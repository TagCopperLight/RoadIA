use crate::map::model::Map;
use axum::extract::ws::{Message, WebSocket};
use serde_json::json;
use tokio::time::{sleep, Duration};



pub async fn websocket_loop(mut socket: WebSocket, map: Map) {
    // 1. Send Map Init
    let roads_data: Vec<_> = map
        .graph
        .raw_edges()
        .iter()
        .map(|edge| {
            let (from_idx, to_idx) = (edge.source(), edge.target());
            let from_id = map.graph[from_idx].id;
            let to_id = map.graph[to_idx].id;
            let road = &edge.weight;
            serde_json::json!({
                "from_id": from_id,
                "to_id": to_id,
                "length_m": road.length_m,
            })
        })
        .collect();

    let nodes_data: Vec<_> = map.graph.node_weights().map(|n| {
            serde_json::json!({
                "id": n.id,
                "name": n.name,
                "x": n.x,
                "y": n.y
            })
    }).collect();

    let init_msg = json!({
        "type": "init",
        "intersections": nodes_data,
        "roads": roads_data
    });
    
    if socket.send(Message::Text(init_msg.to_string())).await.is_err() {
        return;
    }

    loop {
            
        // Capture traffic lights
        let mut lights_data = Vec::new();
        for node in map.graph.node_weights() {
            if !node.traffic_lights.is_empty() {
                 lights_data.push(json!({
                     "intersection_id": node.id,
                     "lights": node.traffic_lights.iter().map(|(id, color)| (*id, format!("{:?}", color))).collect::<std::collections::HashMap<_, _>>()
                 }));
            }
        }

        let update_msg = json!({
            "type": "update",
            "time_s": 0.0, // TODO: pass time via handle if needed
            "lights": lights_data
        });

        if socket.send(Message::Text(update_msg.to_string())).await.is_err() {
            break;
        }

        sleep(Duration::from_millis(100)).await;
    }
}
