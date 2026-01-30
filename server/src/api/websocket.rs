use axum::{
    extract::{ws::{Message, WebSocket, WebSocketUpgrade}, State},
    response::IntoResponse,
};
use serde_json::{json, Value};

use crate::map::model::Map;
use crate::api::server::AppState;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(tag = "id", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum ClientPacket {
    Connect { token: String },
}

#[derive(Debug, Serialize)]
#[serde(tag = "id", content = "data")]
#[serde(rename_all = "camelCase")]
pub enum ServerPacket {
    Map { nodes: Vec<Value>, edges: Vec<Value> },
    VehicleUpdate { vehicles: Vec<Value> },
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_loop(socket, state))
}

async fn ws_loop(mut socket: WebSocket, state: Arc<AppState>) {
    let mut rx = state.tx.subscribe();
    
    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(msg)) => {
                        match msg {
                            Message::Text(text) => {
                                match serde_json::from_str::<ClientPacket>(&text) {
                                    Ok(packet) => {
                                        println!("Received Packet: {:?}", packet);
                                        match packet {
                                            ClientPacket::Connect { token } => {
                                                println!("Client connected with token: {}", token);
                                                let (nodes, edges) = serialize_map(&state.map);
                                                let response = ServerPacket::Map { nodes, edges };
                                                if let Ok(text) = serde_json::to_string(&response) {
                                                    if let Err(e) = socket.send(Message::Text(text)).await {
                                                        println!("Failed to send message: {}", e);
                                                        break;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => println!("Failed to parse packet: {} (text: {})", e, text),
                                }
                            }
                            _ => {
                                println!("Client disconnected");
                                break;
                            }
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
            Ok(msg) = rx.recv() => {
                if let Err(e) = socket.send(Message::Text(msg)).await {
                    println!("Failed to send broadcast message: {}", e);
                    break;
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
