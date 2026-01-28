use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};
use serde_json::{json, Value};

use crate::map::model::Map;

pub async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(ws_loop)
}

async fn ws_loop(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        if let Ok(msg) = msg {
            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<Value>(&text) {
                        Ok(json) => println!("Received JSON: {:?}", json),
                        Err(e) => println!("Failed to parse JSON: {} (text: {})", e, text),
                    }
                }
                _ => {
                    println!("Client disconnected");
                    break;
                }
            }
        } else {
            println!("Client disconnected");
            break;
        }
    }
}

fn serialize_map(map: &Map) -> Value {
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
                "length": r.length_m,
            })
        })
        .collect();

    json!({
        "nodes": nodes,
        "edges": edges
    })
}
