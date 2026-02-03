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
                match msg {
                    Some(Ok(msg)) => {
                        match msg {
                            Message::Text(text) => {
                                match serde_json::from_str::<ClientPacket>(&text) {
                                    Ok(packet) => handle_client_packet(packet, &mut socket, &state).await,
                                    Err(e) => println!("Failed to parse packet: {} (text: {})", e, text),
                                }
                            }
                            Message::Close(_) => {
                                println!("Client disconnected (Close frame)");
                                break;
                            }
                            _ => {}
                        }
                    }
                    Some(Err(e)) => {
                        println!("WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        println!("Client disconnected");
                        break;
                    }
                }
            }
            Ok(packet) = rx.recv() => {
                 if let Ok(text) = serde_json::to_string(&packet) {
                    if let Err(e) = socket.send(Message::Text(text)).await {
                        println!("Failed to send message: {}", e);
                        break;
                    }
                }
            }
        }
    }
    println!("WebSocket loop ended");
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
