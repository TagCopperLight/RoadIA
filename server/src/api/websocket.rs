use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, Query, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::broadcast;

use std::collections::HashSet;
use crate::map::intersection::IntersectionKind;
use crate::map::model::Map;
use crate::map::editor;
use crate::simulation::vehicle::{Vehicle, VehicleKind, VehicleState};
use crate::api::runner::runner::{AppState, SimulationInstance};

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
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "id", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum ServerPacket {
    Map { nodes: Vec<Value>, edges: Vec<Value> },
    VehicleUpdate { vehicles: Vec<Value>, traffic_lights: Vec<Value> },
    MapEdit { success: bool, error: Option<String>, nodes: Vec<Value>, edges: Vec<Value> },
    Score { score: f32, total_trip_time: f32, total_emitted_co2: f32, network_length: f32, total_distance_traveled: f32, success_rate: f32, },
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
    println!("Received Packet: {:?}", packet);
    match packet {
        ClientPacket::StartSimulation {} => {
            println!("Client started simulation");
            instance.controller.start();
        }

        ClientPacket::StopSimulation {} => {
            println!("Client stopped simulation");
            instance.controller.stop();
        }

        ClientPacket::ResetSimulation {} => {
            println!("Client reset simulation");
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

            let map_snapshot = eng.config.map.clone();
            for vehicle in eng.vehicles.iter_mut() {
                let _ = vehicle.update_path(&map_snapshot);
            }
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
        }
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
