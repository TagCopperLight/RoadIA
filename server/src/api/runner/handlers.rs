use std::collections::HashMap;
use std::sync::Arc;
use tokio::io;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tokio::time::Duration;
use axum::{Router, routing::{get, post}, extract::State, Json};
use uuid::Uuid;
use axum::http::{HeaderValue, Method, header::CONTENT_TYPE};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::api::websocket::ws_handler;
use crate::api::runner::map_generator::{create_random_vehicles, create_osm_map};
use super::runner::SimulationInstance;

pub struct AppState {
    pub simulations: Arc<RwLock<HashMap<Uuid, Arc<SimulationInstance>>>>,
}

#[derive(serde::Deserialize)]
pub struct CustomMapRequest {
    pub min_lat: f64,
    pub min_lon: f64,
    pub max_lat: f64,
    pub max_lon: f64,
}

async fn create_custom_simulation_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CustomMapRequest>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    // Validate bounding box
    if payload.min_lat >= payload.max_lat || payload.min_lon >= payload.max_lon {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid bounding box: min must be less than max".to_string()));
    }
    if !(-90.0..=90.0).contains(&payload.min_lat) || !(-90.0..=90.0).contains(&payload.max_lat) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Latitude must be between -90 and 90".to_string()));
    }
    if !(-180.0..=180.0).contains(&payload.min_lon) || !(-180.0..=180.0).contains(&payload.max_lon) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Longitude must be between -180 and 180".to_string()));
    }
    // ~0.09 degrees ≈ 10 km, matching the client-side diagonal limit
    if payload.max_lat - payload.min_lat > 0.09 || payload.max_lon - payload.min_lon > 0.09 {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Bounding box too large. Please select a smaller area (max ~10 km).".to_string()));
    }

    let uuid = Uuid::new_v4();
    let tmp_osm_path = format!("data/tmp/{}.osm", uuid);
    let tmp_pbf_path = format!("data/tmp/{}.osm.pbf", uuid);

    // Ensure the tmp directory exists
    tokio::fs::create_dir_all("data/tmp").await.map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to create tmp directory".to_string()))?;

    let overpass_query = format!(
        "[out:xml][timeout:25][maxsize:1073741824];(way[highway]({min_lat},{min_lon},{max_lat},{max_lon});>;);out body;",
        min_lat = payload.min_lat,
        min_lon = payload.min_lon,
        max_lat = payload.max_lat,
        max_lon = payload.max_lon
    );

    let client = reqwest::Client::builder()
        .user_agent("RoadIA/0.1.0")
        .timeout(Duration::from_secs(35))
        .build()
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to build HTTP client: {}", e)))?;
    let res = client.post("https://overpass-api.de/api/interpreter")
        .body(overpass_query)
        .send()
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to fetch from Overpass API: {}", e)))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        if err_text.contains("too busy") {
            return Err((axum::http::StatusCode::SERVICE_UNAVAILABLE, "Les serveurs d'OpenStreetMap sont actuellement surchargés. Veuillez réessayer plus tard.".to_string()));
        }
        return Err((axum::http::StatusCode::BAD_GATEWAY, format!("Overpass API error: {}", err_text)));
    }

    let map_data = res.text().await.map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to get Overpass text".to_string()))?;

    // Also handle HTML errors that Overpass might return with 200 OK (sometimes they do)
    if map_data.contains("too busy") {
        return Err((axum::http::StatusCode::SERVICE_UNAVAILABLE, "Les serveurs d'OpenStreetMap sont actuellement surchargés. Veuillez réessayer plus tard.".to_string()));
    }

    tokio::fs::write(&tmp_osm_path, map_data).await.map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to write temporary osm file".to_string()))?;

    let osmium_status = tokio::process::Command::new("osmium")
        .args(["cat", &tmp_osm_path, "-o", &tmp_pbf_path])
        .status()
        .await
        .map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to execute osmium".to_string()))?;

    if !osmium_status.success() {
        let _ = tokio::fs::remove_file(&tmp_osm_path).await;
        let _ = tokio::fs::remove_file(&tmp_pbf_path).await;
        return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Osmium failed to convert the file".to_string()));
    }

    let map_result = create_osm_map(&tmp_pbf_path);

    // Clean up temporary files now that the map is loaded in memory
    let _ = tokio::fs::remove_file(&tmp_osm_path).await;
    let _ = tokio::fs::remove_file(&tmp_pbf_path).await;

    let map = map_result.map_err(|e| {
        eprintln!("Failed to parse custom map region: {:?}", e);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse custom mapped region".to_string())
    })?;
    let vehicles = create_random_vehicles(&map, 200);

    let instance = SimulationInstance::new(map, vehicles);
    let token = instance.token.clone();

    state.simulations.write().await.insert(uuid, instance);

    Ok(Json(serde_json::json!({ "uuid": uuid, "token": token })))
}

async fn create_simulation_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let uuid = Uuid::new_v4();
    let instance = SimulationInstance::new_default();
    let token = instance.token.clone();

    state.simulations.write().await.insert(uuid, instance);

    Json(serde_json::json!({ "uuid": uuid, "token": token }))
}

pub async fn run() -> io::Result<()> {
    let shared_state = Arc::new(AppState {
        simulations: Arc::new(RwLock::new(HashMap::new())),
    });

    let allowed_origins: Vec<HeaderValue> = std::env::var("ALLOWED_ORIGINS")
        .expect("ALLOWED_ORIGINS must be set (comma-separated list, e.g. http://localhost:3000)")
        .split(',')
        .filter_map(|o| o.trim().parse().ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(allowed_origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE]);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/simulations", post(create_simulation_handler))
        .route("/api/custom_map", post(create_custom_simulation_handler))
        .layer(cors)
        .with_state(shared_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
