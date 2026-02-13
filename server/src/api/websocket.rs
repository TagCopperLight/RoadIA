use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::map::model::Map;
use crate::api::server::AppState;

#[derive(Debug, Deserialize)]
#[serde(tag = "id", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum ClientPacket {
    Connect { token: String },
    StartSimulation {},
    StopSimulation {},
}

#[derive(Debug, Serialize, Clone)]
#[serde(tag = "id", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum ServerPacket {
    Map { nodes: Vec<Value>, edges: Vec<Value> },
    VehicleUpdate { vehicles: Vec<Value> },
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
            Ok(packet) = rx.recv() => {
                if !process_broadcast_msg(packet, &mut socket).await {
                    break;
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
            let (nodes, edges) = serialize_map(&state.map);
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
    }
}

fn serialize_map(map: &Map) -> (Vec<Value>, Vec<Value>) {
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
            })
        })
        .collect();

    (nodes, edges)
}
