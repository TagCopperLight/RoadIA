use std::sync::Arc;
use tokio::io;
use tokio::net::TcpListener;
use axum::{Router, routing::get};
use crate::map::model::Map;
use crate::api::websocket::ws_handler;

pub struct AppState {
    pub map: Map,
}

pub async fn run() -> io::Result<()> {
    let map = Map::new();
    
    let shared_state = Arc::new(AppState { map });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(shared_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    
    Ok(())
}
