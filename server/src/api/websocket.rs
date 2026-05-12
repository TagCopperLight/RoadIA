use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use tokio::sync::broadcast;

use std::collections::{HashMap, HashSet};
use crate::map::intersection::IntersectionKind;
use crate::map::model::Map;
use crate::map::editor;
use crate::simulation::engine::Simulation;
use crate::simulation::vehicle::{LaneId, TripRequest, Vehicle, VehicleKind, VehicleSpec, VehicleState, VehicleType};
use crate::simulation::engine::BusLine;
use crate::api::runner::runner::SimulationInstance;
use crate::api::runner::handlers::AppState;
use crate::api::runner::map_generator::create_random_vehicles;

#[derive(Debug, Deserialize)]
pub struct ConnectParams {
    pub uuid: String,
    pub token: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "id", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum ClientPacket {
    StartSimulation {},
    StopSimulation {},
    ResetSimulation {},
    AddNode { x: f32, y: f32, kind: String },
    DeleteNode { id: u32 },
    UpdateNode { id: u32, kind: String },
    AddRoad { from_id: u32, to_id: u32, lane_count: u8, speed_limit: f32 },
    DeleteRoad { id: u32 },
    UpdateRoad { id: u32, speed_limit: f32, lane_count: Option<u8> },
    SetSpeed { multiplier: u32 },
    RequestScore {},
    RequestDensity {},
    AddWaypoints { vehicle_id: u64, node_ids: Vec<u32> },
    RequestVehicles {},
    CreateBusLine { name: String, stop_node_ids: Vec<u32> },
    DeleteBusLine { bus_line_id: u64 },
    RequestBusLines {},
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "id", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum ServerPacket {
    Map { nodes: Vec<Value>, edges: Vec<Value> },
    VehicleUpdate { vehicles: Vec<Value>, traffic_lights: Vec<Value> },
    MapEdit { success: bool, error: Option<String>, nodes: Vec<Value>, edges: Vec<Value> },
    Score {
        score: f32,
        total_trip_time: f32,
        ref_total_trip_time: f32,
        total_emitted_co2: f32,
        ref_total_emitted_co2: f32,
        network_length: f32,
        ref_network_length: f32,
        success_rate: f32, },
    SimulationFinished {},
    DensityMap { edges: Vec<Value> },
    VehicleList { vehicles: Vec<Value> },
    BusLineList { bus_lines: Vec<Value> },
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(params): Query<ConnectParams>,
    State(state): State<Arc<AppState>>,
) -> axum::response::Response {
    let parsed_uuid = match Uuid::parse_str(&params.uuid) {
        Ok(u) => u,
        Err(_) => {
            println!("Connection rejected: Invalid UUID format. UUID={}", params.uuid);
            return ws.on_upgrade(|mut socket| async move {
                let _ = socket.send(axum::extract::ws::Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 4001,
                    reason: "Unauthorized".into(),
                }))).await;
            }).into_response();
        }
    };

    let instance = {
        let simulations = state.simulations.read().await;
        simulations.get(&parsed_uuid).cloned()
    };

    match instance {
        Some(instance) if instance.token == params.token => {
            ws.on_upgrade(move |socket| ws_loop(socket, instance, state, parsed_uuid)).into_response()
        }
        _ => {
            println!("Connection rejected: Invalid uuid or token. UUID={}", parsed_uuid);
            ws.on_upgrade(|mut socket| async move {
                let _ = socket.send(axum::extract::ws::Message::Close(Some(axum::extract::ws::CloseFrame {
                    code: 4001,
                    reason: "Unauthorized".into(),
                }))).await;
            }).into_response()
        }
    }
}

async fn ws_loop(
    mut socket: WebSocket,
    instance: Arc<SimulationInstance>,
    state: Arc<AppState>,
    uuid: Uuid,
) {
    instance.active_connections.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let mut rx = instance.broadcast.subscribe();
    println!("New WebSocket client connected");

    // Send initial map state immediately on connect
    {
        let eng = instance.engine.lock().await;
        let (nodes, edges) = serialize_map(&eng.config.map);
        drop(eng);
        let packet = ServerPacket::Map { nodes, edges };
        if let Ok(text) = serde_json::to_string(&packet) {
            if let Err(e) = socket.send(Message::Text(text)).await {
                println!("Failed to send initial map: {}", e);
                return;
            }
        }
    }

    loop {
        tokio::select! {
            msg = socket.recv() => {
                if !process_incoming_msg(msg, &mut socket, &instance).await {
                    break;
                }
            }
            packet = rx.recv() => {
                match packet {
                    Ok(packet) => {
                        if !process_broadcast_msg(packet, &mut socket).await {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }
    if instance.active_connections.fetch_sub(1, std::sync::atomic::Ordering::Relaxed) == 1 {
        // Last client disconnected, stop the simulation and remove the instance.
        instance.controller.stop();
        state.simulations.write().await.remove(&uuid);
        println!("Last client disconnected, simulation {} removed", uuid);
    }
    println!("WebSocket loop ended");
}

async fn process_incoming_msg(
    msg: Option<Result<Message, axum::Error>>,
    socket: &mut WebSocket,
    instance: &Arc<SimulationInstance>,
) -> bool {
    match msg {
        Some(Ok(msg)) => match msg {
            Message::Text(text) => {
                match serde_json::from_str::<ClientPacket>(&text) {
                    Ok(packet) => handle_client_packet(packet, socket, instance).await,
                    Err(e) => println!("Failed to parse packet: {} (text: {})", e, text),
                }
                true
            }
            Message::Close(_) => {
                println!("Client disconnected (Close frame)");
                false
            }
            _ => true,
        },
        Some(Err(e)) => {
            println!("WebSocket error: {}", e);
            false
        }
        None => {
            println!("Client disconnected");
            false
        }
    }
}

async fn process_broadcast_msg(packet: ServerPacket, socket: &mut WebSocket) -> bool {
    if let Ok(text) = serde_json::to_string(&packet) {
        if let Err(e) = socket.send(Message::Text(text)).await {
            println!("Failed to send message: {}", e);
            return false;
        }
    }
    true
}

async fn handle_client_packet(
    packet: ClientPacket,
    socket: &mut WebSocket,
    instance: &Arc<SimulationInstance>,
) {
    match packet {
        ClientPacket::StartSimulation {} => {
            instance.controller.start();
        }

        ClientPacket::RequestScore {} => {
            let broadcast = instance.broadcast.clone();
            let engine = instance.initial_engine.clone();
            tokio::spawn(async move {
                let sim_clone = {
                    let eng = engine.lock().await;
                    eng.clone()
                };

                let score = tokio::task::spawn_blocking(move || {
                    let mut sim = sim_clone;
                    sim.run();
                    sim.get_score()
                }).await.expect("score computation panicked");

                let _ = broadcast.send(ServerPacket::Score {
                    score: score.score,
                    total_trip_time: score.total_trip_time,
                    ref_total_trip_time: score.ref_total_trip_time,
                    total_emitted_co2: score.total_emitted_co2,
                    ref_total_emitted_co2: score.ref_total_emitted_co2,
                    network_length: score.network_length,
                    ref_network_length: score.ref_network_length,
                    success_rate: score.success_rate,
                });
            });
        }

        ClientPacket::RequestDensity {} => {
            let broadcast = instance.broadcast.clone();
            let engine = instance.initial_engine.clone();
            tokio::spawn(async move {
                let sim_clone = {
                    let eng = engine.lock().await;
                    eng.clone()
                };

                let edges = tokio::task::spawn_blocking(move || {
                    let mut sim = sim_clone;
                    let mut per_edge: HashMap<_, (f32, usize)> = HashMap::new();

                    while !sim.all_vehicles_arrived && sim.current_time < sim.config.end_time {
                        sim.step();
                        sim.current_time += sim.config.time_step;

                        for (lane_id, vehicle_indices) in &sim.vehicles_by_lane {
                            let edge_idx = match lane_id {
                                LaneId::Normal(edge, _) => *edge,
                                LaneId::Internal(_, _) => continue,
                            };
                            for &vidx in vehicle_indices {
                                let vel = sim.vehicles[vidx].velocity;
                                let e = per_edge.entry(edge_idx).or_insert((0.0, 0));
                                e.0 += vel;
                                e.1 += 1;
                            }
                        }
                    }

                    per_edge.into_iter().filter_map(|(edge_idx, (sum_vel, count))| {
                        if count == 0 { return None; }
                        let road = &sim.config.map.graph[edge_idx];
                        let avg_vel = sum_vel / count as f32;
                        let speed_ratio = if road.speed_limit > 0.0 {
                            (avg_vel / road.speed_limit).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        Some(json!({ "id": road.id, "speed_ratio": speed_ratio }))
                    }).collect::<Vec<_>>()
                }).await.expect("density computation panicked");

                let _ = broadcast.send(ServerPacket::DensityMap { edges });
            });
        }

        ClientPacket::StopSimulation {} => {
            instance.controller.stop();
        }

        ClientPacket::ResetSimulation {} => {
            instance.controller.stop();
            let mut eng = instance.engine.lock().await;
            eng.current_time = 0.0;
            eng.vehicles_by_lane.clear();
            eng.link_states.clear();

            for vehicle in &mut eng.vehicles {
                vehicle.state = VehicleState::WaitingToDepart;
                vehicle.position_on_lane = 0.0;
                vehicle.velocity = 0.0;
                vehicle.previous_velocity = 0.0;
                vehicle.path = Vec::new();
                vehicle.path_index = 0;
                vehicle.current_lane = None;
                vehicle.drive_plan = Vec::new();
                vehicle.registered_link_ids = Vec::new();
                vehicle.waiting_time = 0.0;
                vehicle.impatience = 0.0;
            }

            eng.config.end_time = eng.config.map.settings.simulation_duration;
            let new_count = eng.config.map.settings.vehicle_count;
            let map_snapshot = eng.config.map.clone();
            eng.vehicles = create_random_vehicles(&map_snapshot, new_count);
            eng.next_vehicle_id = eng.vehicles.len() as u64;
            eng.all_vehicles_arrived = false;
            for vehicle in eng.vehicles.iter_mut() {
                vehicle.update_path(&map_snapshot);
            }

            // Recreate bus vehicles from saved bus lines (preserves user's bus lines across resets)
            eng.bus_lines.clear();
            eng.next_bus_line_id = map_snapshot.next_bus_line_id;
            for bl in &map_snapshot.bus_lines {
                eng.add_bus_from_saved(bl);
            }

            let vehicles: Vec<Value> = eng.vehicles.iter()
                .map(|v| serialize_vehicle_summary(v, &map_snapshot))
                .collect();
            let bus_lines: Vec<Value> = eng.bus_lines.iter().map(serialize_bus_line).collect();
            let snapshot = eng.clone();
            drop(eng);
            *instance.initial_engine.lock().await = snapshot;
            let _ = instance.broadcast.send(ServerPacket::VehicleList { vehicles });
            let _ = instance.broadcast.send(ServerPacket::BusLineList { bus_lines });
        }

        ClientPacket::SetSpeed { multiplier } => {
            let clamped = multiplier.clamp(1, 20);
            instance.speed_multiplier.store(clamped, Ordering::Relaxed);
        }

        ClientPacket::AddNode { x, y, kind } => {
            if instance.controller.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let kind = match serialize_intersection_kind(&kind) {
                Ok(k) => k,
                Err(e) => { send_edit_error(socket, &e).await; return; }
            };
            let mut eng = instance.engine.lock().await;
            editor::add_node(&mut eng.config.map, x, y, kind);
            let (nodes, edges) = serialize_map(&eng.config.map);
            drop(eng);
            broadcast_map_edit_success(&instance.broadcast, nodes, edges);
        }

        ClientPacket::DeleteNode { id } => {
            if instance.controller.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let mut eng = instance.engine.lock().await;
            match editor::delete_node(&mut eng.config.map, id) {
                Ok(()) => {
                    let (nodes, edges) = serialize_map(&eng.config.map);
                    drop(eng);
                    broadcast_map_edit_success(&instance.broadcast, nodes, edges);
                }
                Err(e) => {
                    drop(eng);
                    send_edit_error(socket, &e).await;
                }
            }
        }


        ClientPacket::UpdateNode { id, kind } => {
            if instance.controller.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let kind = match serialize_intersection_kind(&kind) {
                Ok(k) => k,
                Err(e) => { send_edit_error(socket, &e).await; return; }
            };
            let mut eng = instance.engine.lock().await;
            match editor::update_node(&mut eng.config.map, id, kind) {
                Ok(()) => {
                    let (nodes, edges) = serialize_map(&eng.config.map);
                    drop(eng);
                    broadcast_map_edit_success(&instance.broadcast, nodes, edges);
                }
                Err(e) => {
                    drop(eng);
                    send_edit_error(socket, &e).await;
                }
            }
        }

        ClientPacket::AddRoad { from_id, to_id, lane_count, speed_limit } => {
            if instance.controller.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let mut eng = instance.engine.lock().await;
            match editor::add_road(&mut eng.config.map, from_id, to_id, lane_count, speed_limit) {
                Ok(_road_id) => {
                    let (nodes, edges) = serialize_map(&eng.config.map);
                    drop(eng);
                    broadcast_map_edit_success(&instance.broadcast, nodes, edges);
                }
                Err(e) => {
                    drop(eng);
                    send_edit_error(socket, &e).await;
                }
            }
        }

        ClientPacket::DeleteRoad { id } => {
            if instance.controller.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let mut eng = instance.engine.lock().await;
            match editor::delete_road(&mut eng.config.map, id) {
                Ok(()) => {
                    let (nodes, edges) = serialize_map(&eng.config.map);
                    drop(eng);
                    broadcast_map_edit_success(&instance.broadcast, nodes, edges);
                }
                Err(e) => {
                    drop(eng);
                    send_edit_error(socket, &e).await;
                }
            }
        }

        ClientPacket::UpdateRoad { id, speed_limit, lane_count } => {
            if instance.controller.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let mut eng = instance.engine.lock().await;
            match editor::update_road(&mut eng.config.map, id, speed_limit, lane_count) {
                Ok(()) => {
                    let (nodes, edges) = serialize_map(&eng.config.map);
                    drop(eng);
                    broadcast_map_edit_success(&instance.broadcast, nodes, edges);
                }
                Err(e) => {
                    drop(eng);
                    send_edit_error(socket, &e).await;
                }
            }
        }

        ClientPacket::AddWaypoints { vehicle_id, node_ids } => {
            let mut eng = instance.engine.lock().await;
            let map = eng.config.map.clone();
            let waypoints: Vec<_> = node_ids
                .iter()
                .filter_map(|nid| map.node_index_map.get(nid).copied())
                .collect();
            if let Some(v) = eng.vehicles.iter_mut().find(|v| v.id == vehicle_id) {
                v.waypoints = waypoints;
                v.update_path(&map);
            }
            let vehicles: Vec<Value> = eng.vehicles.iter()
                .map(|v| serialize_vehicle_summary(v, &map))
                .collect();
            drop(eng);
            let _ = instance.broadcast.send(ServerPacket::VehicleList { vehicles });
        }

        ClientPacket::RequestVehicles {} => {
            let eng = instance.engine.lock().await;
            let vehicles: Vec<Value> = eng.vehicles.iter()
                .map(|v| serialize_vehicle_summary(v, &eng.config.map))
                .collect();
            drop(eng);
            let packet = ServerPacket::VehicleList { vehicles };
            if let Ok(text) = serde_json::to_string(&packet) {
                let _ = socket.send(Message::Text(text)).await;
            }
        }

        ClientPacket::CreateBusLine { name, stop_node_ids } => {
            if stop_node_ids.len() < 2 {
                return;
            }
            let mut eng = instance.engine.lock().await;
            let map = eng.config.map.clone();

            let stops: Vec<_> = stop_node_ids.iter()
                .filter_map(|nid| map.node_index_map.get(nid).copied())
                .collect();
            if stops.len() < 2 {
                return;
            }

            let spec = VehicleSpec::new(VehicleKind::Bus, 8.0, 2.0, 3.0, 1.0, 12.0);
            let trip = TripRequest { origin: stops[0], destination: stops[0], departure_time: 0.0 };
            let vehicle_id = eng.next_vehicle_id;
            eng.next_vehicle_id += 1;

            let mut bus = Vehicle::new(vehicle_id, spec, trip, VehicleType::Electrique);
            bus.waypoints = stops[1..].to_vec();
            bus.update_path(&map);

            if bus.path.len() < 2 {
                return;
            }

            eng.vehicles.push(bus);

            let bus_line_id = eng.next_bus_line_id;
            eng.next_bus_line_id += 1;
            eng.bus_lines.push(BusLine { id: bus_line_id, name: name.clone(), stop_node_ids: stop_node_ids.clone(), vehicle_id });
            eng.config.map.bus_lines.push(crate::map::model::SavedBusLine { id: bus_line_id, name, stop_node_ids });
            eng.config.map.next_bus_line_id = eng.next_bus_line_id;

            let bus_lines: Vec<Value> = eng.bus_lines.iter().map(serialize_bus_line).collect();
            drop(eng);
            let _ = instance.broadcast.send(ServerPacket::BusLineList { bus_lines });
        }

        ClientPacket::DeleteBusLine { bus_line_id } => {
            let mut eng = instance.engine.lock().await;
            if let Some(pos) = eng.bus_lines.iter().position(|bl| bl.id == bus_line_id) {
                let vehicle_id = eng.bus_lines[pos].vehicle_id;
                eng.bus_lines.remove(pos);
                eng.config.map.bus_lines.retain(|bl| bl.id != bus_line_id);
                if let Some(v) = eng.vehicles.iter_mut().find(|v| v.id == vehicle_id) {
                    v.state = VehicleState::Arrived;
                }
            }
            let bus_lines: Vec<Value> = eng.bus_lines.iter().map(serialize_bus_line).collect();
            drop(eng);
            let _ = instance.broadcast.send(ServerPacket::BusLineList { bus_lines });
        }

        ClientPacket::RequestBusLines {} => {
            let eng = instance.engine.lock().await;
            let bus_lines: Vec<Value> = eng.bus_lines.iter().map(serialize_bus_line).collect();
            drop(eng);
            let packet = ServerPacket::BusLineList { bus_lines };
            if let Ok(text) = serde_json::to_string(&packet) {
                let _ = socket.send(Message::Text(text)).await;
            }
        }
    }
}

async fn send_edit_error(socket: &mut WebSocket, error: &str) {
    let packet = ServerPacket::MapEdit {
        success: false,
        error: Some(error.to_string()),
        nodes: vec![],
        edges: vec![],
    };
    if let Ok(text) = serde_json::to_string(&packet) {
        let _ = socket.send(Message::Text(text)).await;
    }
}

fn broadcast_map_edit_success(
    broadcast: &broadcast::Sender<ServerPacket>,
    nodes: Vec<Value>,
    edges: Vec<Value>,
) {
    let packet = ServerPacket::MapEdit {
        success: true,
        error: None,
        nodes,
        edges,
    };
    let _ = broadcast.send(packet);
}

pub fn serialize_map(map: &Map) -> (Vec<Value>, Vec<Value>) {
    let nodes: Vec<Value> = map
        .graph
        .node_indices()
        .map(|i| {
            let n = &map.graph[i];
            let has_traffic_light = map.traffic_lights
                .values()
                .any(|c| c.intersection_id == n.id);
            let internal_lanes: Vec<Value> = n.internal_lanes.iter().map(|lane| {
                let link_type = map.graph.edge_indices()
                    .flat_map(|e| map.graph[e].lanes.iter())
                    .flat_map(|l| l.links.iter())
                    .find(|link| link.via_internal_lane_id == lane.id)
                    .map(|link| format!("{:?}", link.link_type))
                    .unwrap_or_else(|| "Priority".to_string());
                json!({
                    "id": lane.id,
                    "entry": [lane.entry.0, lane.entry.1],
                    "exit": [lane.exit.0, lane.exit.1],
                    "link_type": link_type,
                })
            }).collect();
            json!({
                "id": n.id,
                "kind": format!("{:?}", n.kind),
                "x": n.center_coordinates.x,
                "y": n.center_coordinates.y,
                "has_traffic_light": has_traffic_light,
                "radius": n.radius,
                "internal_lanes": internal_lanes,
            })
        })
        .collect();

    let edges: Vec<Value> = map
        .graph
        .edge_indices()
        .map(|e| {
            let (a, b) = map
                .graph
                .edge_endpoints(e)
                .expect("edge_endpoints returned None contextually");
            let r = &map.graph[e];
            json!({
                "id": r.id,
                "from": map.graph[a].id,
                "to": map.graph[b].id,
                "lane_count": r.lanes.len(),
                "lane_width": r.lane_width,
                "length": r.length,
                "speed_limit": r.speed_limit,
            })
        })
        .collect();

    (nodes, edges)
}

pub fn serialize_vehicle(vehicle: &Vehicle, sim_map: &Map) -> Value {
    let coords = vehicle.get_coordinates(sim_map);
    let heading = vehicle.get_heading(sim_map);
    let waypoint_ids: Vec<u32> = vehicle.waypoints.iter()
        .map(|&ni| sim_map.graph[ni].id)
        .collect();
    json!({
        "id": vehicle.id,
        "x": coords.x,
        "y": coords.y,
        "heading": heading,
        "kind": match vehicle.spec.kind {
                VehicleKind::Car => "Car",
                VehicleKind::Bus => "Bus",
        },
        "state": match vehicle.state {
            VehicleState::WaitingToDepart => "Waiting",
            VehicleState::OnRoad => "Moving",
            VehicleState::Arrived => "Arrived",
        },
        "motorization": match vehicle.motorization {
            VehicleType::Hybride    => "Hybride",
            VehicleType::Electrique => "Electrique",
            VehicleType::Essence    => "Essence",
            VehicleType::Diesel     => "Diesel",
        },
        "origin_id": sim_map.graph[vehicle.trip.origin].id,
        "destination_id": sim_map.graph[vehicle.trip.destination].id,
        "waypoint_ids": waypoint_ids,
    })
}

pub fn serialize_vehicle_summary(vehicle: &Vehicle, map: &Map) -> Value {
    let waypoint_ids: Vec<u32> = vehicle.waypoints.iter()
        .map(|&ni| map.graph[ni].id)
        .collect();
    json!({
        "id": vehicle.id,
        "origin_id": map.graph[vehicle.trip.origin].id,
        "destination_id": map.graph[vehicle.trip.destination].id,
        "motorization": match vehicle.motorization {
            VehicleType::Hybride    => "Hybride",
            VehicleType::Electrique => "Electrique",
            VehicleType::Essence    => "Essence",
            VehicleType::Diesel     => "Diesel",
        },
        "waypoint_ids": waypoint_ids,
    })
}

fn serialize_bus_line(bl: &BusLine) -> Value {
    json!({
        "id": bl.id,
        "name": bl.name,
        "stop_node_ids": bl.stop_node_ids,
        "vehicle_id": bl.vehicle_id,
    })
}

fn serialize_intersection_kind(s: &str) -> Result<IntersectionKind, String> {
    match s {
        "Habitation" => Ok(IntersectionKind::Habitation),
        "Intersection" => Ok(IntersectionKind::Intersection),
        "Workplace" => Ok(IntersectionKind::Workplace),
        other => Err(format!("Unknown intersection kind: {}", other)),
    }
}

pub fn serialize_traffic_lights(map: &Map, green_links: &HashSet<u32>) -> Vec<Value> {
    map.traffic_lights
        .values()
        .map(|controller| {
            let green_road_ids: Vec<u32> = map
                .graph
                .edge_indices()
                .filter_map(|e| {
                    let road = &map.graph[e];
                    let is_green = road.lanes.iter().any(|lane| {
                        lane.links.iter().any(|link| {
                            green_links.contains(&link.id)
                                && map
                                    .graph
                                    .edge_endpoints(e)
                                    .map(|(_, to)| map.graph[to].id == controller.intersection_id)
                                    .unwrap_or(false)
                        })
                    });
                    if is_green { Some(road.id) } else { None }
                })
                .collect();

            json!({
                "id": controller.intersection_id,
                "green_road_ids": green_road_ids
            })
        })
        .collect()
}
