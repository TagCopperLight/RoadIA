use crate::map::model::Map;
use crate::simulation::handle::Handle;
use axum::extract::ws::{Message, WebSocket};
use serde::Serialize;
use tokio::time::{sleep, Duration};

#[derive(Serialize)]
pub struct VehicleUpdate {
    pub id: u64,
    pub x: f32,
    pub y: f32,
    pub state: String,
}

pub async fn websocket_loop(mut socket: WebSocket, handle: Handle, map: &Map) {
    loop {
        let vehicles = handle.snapshot_vehicles();

        let updates: Vec<VehicleUpdate> = vehicles
            .into_iter()
            .map(|a| VehicleUpdate {
                id: a.id,
                x: a.get_coordinates(map).x,
                y: a.get_coordinates(map).y,
                state: format!("{:?}", a.state),
            })
            .collect();

        let json = serde_json::to_string(&updates).unwrap();

        if socket.send(Message::Text(json)).await.is_err() {
            break;
        }

        sleep(Duration::from_millis(100)).await;
    }
}
