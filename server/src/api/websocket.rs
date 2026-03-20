use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::map::intersection::IntersectionKind;
use crate::map::model::Map;
use crate::map::editor;
use crate::simulation::vehicle::{Vehicle, VehicleKind, VehicleState};
use crate::api::runner::runner::AppState;

#[derive(Debug, Deserialize)]
#[serde(tag = "id", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum ClientPacket {
    Connect { token: String },
    StartSimulation {},
    StopSimulation {},
    ResetSimulation {},
    AddNode { x: f32, y: f32, kind: String, name: String },
    DeleteNode { id: u32 },
    MoveNode { id: u32, x: f32, y: f32 },
    UpdateNode { id: u32, kind: String, name: String },
    AddRoad { from_id: u32, to_id: u32, lane_count: u8, speed_limit: f32 },
    DeleteRoad { id: u32 },
    UpdateRoad { id: u32, lane_count: u8, speed_limit: f32, is_blocked: bool, can_overtake: bool },
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "id", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum ServerPacket {
    Map { nodes: Vec<Value>, edges: Vec<Value> },
    VehicleUpdate { vehicles: Vec<Value> },
    MapEdit { success: bool, error: Option<String>, nodes: Vec<Value>, edges: Vec<Value> },
}

pub struct WebSocketService {
    sender: broadcast::Sender<ServerPacket>,
}

impl WebSocketService {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(100);
        Self { sender }
    }

    pub fn send(&self, packet: ServerPacket) {
        let _ = self.sender.send(packet);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<ServerPacket> {
        self.sender.subscribe()
    }
}

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_loop(socket, state))
}

async fn ws_loop(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.websocket_service.subscribe();
    println!("New WebSocket client connected");

    loop {
        tokio::select! {
            msg = socket.recv() => {
                if !process_incoming_msg(msg, &mut socket, &state).await {
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
    println!("WebSocket loop ended");
}

async fn process_incoming_msg(
    msg: Option<Result<Message, axum::Error>>,
    socket: &mut WebSocket,
    state: &Arc<AppState>,
) -> bool {
    match msg {
        Some(Ok(msg)) => match msg {
            Message::Text(text) => {
                match serde_json::from_str::<ClientPacket>(&text) {
                    Ok(packet) => handle_client_packet(packet, socket, state).await,
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
    state: &Arc<AppState>,
) {
    println!("Received Packet: {:?}", packet);
    match packet {
        ClientPacket::Connect { token } => {
            println!("Client connected with token: {}", token);
            let eng = state.engine.lock().await;
            let (nodes, edges) = serialize_map(&eng.config.map);
            drop(eng);
            let response = ServerPacket::Map { nodes, edges };
            if let Ok(text) = serde_json::to_string(&response) {
                if let Err(e) = socket.send(Message::Text(text)).await {
                    println!("Failed to send initial map: {}", e);
                }
            }
        }

        ClientPacket::StartSimulation {} => {
            println!("Client started simulation");
            state.simulation.start();
        }

        ClientPacket::StopSimulation {} => {
            println!("Client stopped simulation");
            state.simulation.stop();
        }

        ClientPacket::ResetSimulation {} => {
            println!("Client reset simulation");
            state.simulation.stop();
            let mut eng = state.engine.lock().await;
            eng.current_time = 0.0;
            eng.vehicles_by_road.clear();

            // Clear all intersection requests from the map nodes.
            for node_idx in eng.config.map.graph.node_indices().collect::<Vec<_>>() {
                eng.config.map.graph[node_idx].requests.clear();
                eng.config.map.graph[node_idx].traffic_order.clear();
            }

            // Reset each vehicle to its initial state.
            for vehicle in &mut eng.vehicles {
                vehicle.state = VehicleState::WaitingToDepart;
                vehicle.position_on_road = 0.0;
                vehicle.previous_position = 0.0;
                vehicle.velocity = 0.0;
                vehicle.previous_velocity = 0.0;
                vehicle.path = Vec::new();
                vehicle.path_index = 0;
            }

            // Re-initialize paths with current map.
            let map_snapshot = eng.config.map.clone();
            for vehicle in &mut eng.vehicles {
                vehicle.update_path(&map_snapshot);
            }
        }

        // Map editing packets — require simulation to be stopped.
        ClientPacket::AddNode { x, y, kind, name } => {
            if state.simulation.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let kind = match serialize_intersection_kind(&kind) {
                Ok(k) => k,
                Err(e) => { send_edit_error(socket, &e).await; return; }
            };
            let mut eng = state.engine.lock().await;
            editor::add_node(&mut eng.config.map, x, y, kind, name);
            let (nodes, edges) = serialize_map(&eng.config.map);
            drop(eng);
            broadcast_map_edit_success(&state.websocket_service, nodes, edges);
        }

        ClientPacket::DeleteNode { id } => {
            if state.simulation.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let mut eng = state.engine.lock().await;
            match editor::delete_node(&mut eng.config.map, id) {
                Ok(()) => {
                    let (nodes, edges) = serialize_map(&eng.config.map);
                    drop(eng);
                    broadcast_map_edit_success(&state.websocket_service, nodes, edges);
                }
                Err(e) => {
                    drop(eng);
                    send_edit_error(socket, &e).await;
                }
            }
        }

        ClientPacket::MoveNode { id, x, y } => {
            if state.simulation.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let mut eng = state.engine.lock().await;
            match editor::move_node(&mut eng.config.map, id, x, y) {
                Ok(()) => {
                    let (nodes, edges) = serialize_map(&eng.config.map);
                    drop(eng);
                    broadcast_map_edit_success(&state.websocket_service, nodes, edges);
                }
                Err(e) => {
                    drop(eng);
                    send_edit_error(socket, &e).await;
                }
            }
        }

        ClientPacket::UpdateNode { id, kind, name } => {
            if state.simulation.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let kind = match serialize_intersection_kind(&kind) {
                Ok(k) => k,
                Err(e) => { send_edit_error(socket, &e).await; return; }
            };
            let mut eng = state.engine.lock().await;
            match editor::update_node(&mut eng.config.map, id, kind, name) {
                Ok(()) => {
                    let (nodes, edges) = serialize_map(&eng.config.map);
                    drop(eng);
                    broadcast_map_edit_success(&state.websocket_service, nodes, edges);
                }
                Err(e) => {
                    drop(eng);
                    send_edit_error(socket, &e).await;
                }
            }
        }

        ClientPacket::AddRoad { from_id, to_id, lane_count, speed_limit } => {
            if state.simulation.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let mut eng = state.engine.lock().await;
            match editor::add_road(&mut eng.config.map, from_id, to_id, lane_count, speed_limit) {
                Ok(_road_id) => {
                    let (nodes, edges) = serialize_map(&eng.config.map);
                    drop(eng);
                    broadcast_map_edit_success(&state.websocket_service, nodes, edges);
                }
                Err(e) => {
                    drop(eng);
                    send_edit_error(socket, &e).await;
                }
            }
        }

        ClientPacket::DeleteRoad { id } => {
            if state.simulation.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let mut eng = state.engine.lock().await;
            match editor::delete_road(&mut eng.config.map, id) {
                Ok(()) => {
                    let (nodes, edges) = serialize_map(&eng.config.map);
                    drop(eng);
                    broadcast_map_edit_success(&state.websocket_service, nodes, edges);
                }
                Err(e) => {
                    drop(eng);
                    send_edit_error(socket, &e).await;
                }
            }
        }

        ClientPacket::UpdateRoad { id, lane_count, speed_limit, is_blocked, can_overtake } => {
            if state.simulation.is_running() {
                send_edit_error(socket, "Stop simulation before editing the map").await;
                return;
            }
            let mut eng = state.engine.lock().await;
            match editor::update_road(&mut eng.config.map, id, lane_count, speed_limit, is_blocked, can_overtake) {
                Ok(()) => {
                    let (nodes, edges) = serialize_map(&eng.config.map);
                    drop(eng);
                    broadcast_map_edit_success(&state.websocket_service, nodes, edges);
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
    ws_service: &WebSocketService,
    nodes: Vec<Value>,
    edges: Vec<Value>,
) {
    let packet = ServerPacket::MapEdit {
        success: true,
        error: None,
        nodes,
        edges,
    };
    ws_service.send(packet);
}

pub fn serialize_map(map: &Map) -> (Vec<Value>, Vec<Value>) {
    let nodes: Vec<Value> = map
        .graph
        .node_indices()
        .map(|i| {
            let n = &map.graph[i];
            json!({
                "id": n.id,
                "kind": format!("{:?}", n.kind),
                "name": n.name,
                "x": n.x,
                "y": n.y
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
                "lane_count": r.lane_count,
                "length": r.length,
                "speed_limit": r.speed_limit,
                "is_blocked": r.is_blocked,
                "can_overtake": r.can_overtake,
            })
        })
        .collect();

    (nodes, edges)
}

pub fn serialize_vehicle(vehicle: &Vehicle, sim_map: &Map) -> Value {
    let coords = vehicle.get_coordinates(sim_map);
    json!({
        "id": vehicle.id,
        "x": coords.x,
        "y": coords.y,
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
