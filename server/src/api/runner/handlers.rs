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
use crate::api::runner::map_generator::create_osm_map;
use crate::map::model::MapSettings;
use crate::simulation::config::ScoreWeights;
use super::runner::SimulationInstance;

fn validate_map_settings(settings: &MapSettings) -> Result<(), (axum::http::StatusCode, String)> {
    if !(0.0..=MapSettings::MAX_SIMULATION_START_TIME_S).contains(&settings.simulation_start_time) {
        return Err((axum::http::StatusCode::BAD_REQUEST, format!(
            "Simulation start time must be between 0 and {} seconds",
            MapSettings::MAX_SIMULATION_START_TIME_S as u32
        )));
    }
    if !(0.0..=1.0).contains(&settings.time_step) || settings.time_step == 0.0 {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Time step must be greater than 0 and at most 1 second".to_string()));
    }

    let weight_sum = settings.time_weight + settings.success_weight + settings.pollution_weight + settings.infrastructure_weight;
    if (weight_sum - 1.0).abs() > 0.01 {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Score weights must sum to 1.0".to_string()));
    }

    Ok(())
}

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
    if payload.min_lat >= payload.max_lat || payload.min_lon >= payload.max_lon {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Invalid bounding box: min must be less than max".to_string()));
    }
    if !(-90.0..=90.0).contains(&payload.min_lat) || !(-90.0..=90.0).contains(&payload.max_lat) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Latitude must be between -90 and 90".to_string()));
    }
    if !(-180.0..=180.0).contains(&payload.min_lon) || !(-180.0..=180.0).contains(&payload.max_lon) {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Longitude must be between -180 and 180".to_string()));
    }
    if payload.max_lat - payload.min_lat > 0.09 || payload.max_lon - payload.min_lon > 0.09 {
        return Err((axum::http::StatusCode::BAD_REQUEST, "Bounding box too large. Please select a smaller area (max ~10 km).".to_string()));
    }

    let uuid = Uuid::new_v4();
    let tmp_osm_path = format!("data/tmp/{}.osm", uuid);
    let tmp_pbf_path = format!("data/tmp/{}.osm.pbf", uuid);

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

    let _ = tokio::fs::remove_file(&tmp_osm_path).await;
    let _ = tokio::fs::remove_file(&tmp_pbf_path).await;

    let map = map_result.map_err(|e| {
        eprintln!("Failed to parse custom map region: {:?}", e);
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse custom mapped region".to_string())
    })?;
    let instance = SimulationInstance::new(map);
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

#[derive(serde::Deserialize)]
struct SaveMapRequest {
    uuid: Uuid,
    token: String,
    name: String,
    #[serde(default)]
    file_uuid: Option<Uuid>,
}

async fn save_map_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SaveMapRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let simulations = state.simulations.read().await;
    let instance = simulations.get(&payload.uuid).ok_or(axum::http::StatusCode::NOT_FOUND)?;

    if instance.token != payload.token {
        return Err(axum::http::StatusCode::UNAUTHORIZED);
    }

    let file_uuid = payload.file_uuid.unwrap_or_else(Uuid::new_v4);
    let path = format!("data/{}.json", file_uuid);

    {
        let mut engine = instance.engine.lock().await;
        engine.config.map.name = payload.name;
        if let Err(e) = engine.config.map.save(&path) {
            println!("Failed to save map: {:?}", e);
            return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    *instance.file_uuid.lock().unwrap() = Some(file_uuid);

    Ok(Json(serde_json::json!({ "status": "success", "file_uuid": file_uuid })))
}

#[derive(serde::Deserialize)]
struct LoadMapRequest {
    file_uuid: Uuid,
}

async fn load_map_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoadMapRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let path = format!("data/{}.json", payload.file_uuid);
    match SimulationInstance::from_file(&path, payload.file_uuid) {
        Ok(instance) => {
            let sim_uuid = Uuid::new_v4();
            let token = instance.token.clone();
            let name = instance.engine.lock().await.config.map.name.clone();
            state.simulations.write().await.insert(sim_uuid, instance);
            Ok(Json(serde_json::json!({ "uuid": sim_uuid, "token": token, "name": name })))
        }
        Err(e) => {
            println!("Failed to load map from {}: {:?}", path, e);
            Err(axum::http::StatusCode::NOT_FOUND)
        }
    }
}

#[derive(serde::Deserialize)]
struct RenameMapRequest {
    file_uuid: Uuid,
    new_name: String,
}

async fn rename_map_handler(
    Json(payload): Json<RenameMapRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let path = format!("data/{}.json", payload.file_uuid);
    let mut map = crate::map::model::Map::load(&path).map_err(|e| {
        println!("Failed to load map for rename: {:?}", e);
        axum::http::StatusCode::NOT_FOUND
    })?;
    map.name = payload.new_name;
    map.save(&path).map_err(|e| {
        println!("Failed to save map after rename: {:?}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({ "status": "success" })))
}

#[derive(serde::Deserialize)]
struct DeleteMapRequest {
    file_uuid: Uuid,
}

async fn delete_map_handler(
    Json(payload): Json<DeleteMapRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let path = format!("data/{}.json", payload.file_uuid);
    std::fs::remove_file(&path).map_err(|e| {
        println!("Failed to delete map: {:?}", e);
        axum::http::StatusCode::NOT_FOUND
    })?;
    Ok(Json(serde_json::json!({ "status": "success" })))
}

async fn list_maps_handler() -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let data_dir = "data";
    let mut maps = Vec::new();

    if let Ok(entries) = std::fs::read_dir(data_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("json") {
                continue;
            }
            let stem = match path.file_stem().and_then(|s| s.to_str()) {
                Some(s) => s.to_string(),
                None => continue,
            };
            let file_uuid = match Uuid::parse_str(&stem) {
                Ok(u) => u,
                Err(_) => continue,
            };
            if let Ok(map) = crate::map::model::Map::load(&path) {
                maps.push(serde_json::json!({ "uuid": file_uuid, "name": map.name }));
            }
        }
    }

    Ok(Json(serde_json::json!({ "maps": maps })))
}

async fn get_simulation_settings_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(uuid): axum::extract::Path<Uuid>,
) -> Result<Json<MapSettings>, axum::http::StatusCode> {
    let simulations = state.simulations.read().await;
    let instance = simulations.get(&uuid).ok_or(axum::http::StatusCode::NOT_FOUND)?;
    let engine = instance.engine.lock().await;
    Ok(Json(engine.config.map.settings.clone()))
}

async fn update_simulation_settings_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(uuid): axum::extract::Path<Uuid>,
    Json(payload): Json<MapSettings>,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, String)> {
    validate_map_settings(&payload)?;

    let simulations = state.simulations.read().await;
    let instance = simulations.get(&uuid).ok_or_else(|| (axum::http::StatusCode::NOT_FOUND, "Simulation not found".to_string()))?;

    let file_uuid = instance.file_uuid.lock().unwrap().clone();

    {
        let mut engine = instance.engine.lock().await;
        engine.config.score_weights = ScoreWeights::from_settings(&payload);
        engine.config.map.settings = payload;

        if let Some(fid) = file_uuid {
            let path = format!("data/{}.json", fid);
            if let Err(e) = engine.config.map.save(&path) {
                println!("Failed to persist settings: {:?}", e);
                return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to persist settings to disk".to_string()));
            }
        }
    }

    Ok(Json(serde_json::json!({ "status": "success" })))
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
        .allow_headers([CONTENT_TYPE, axum::http::header::AUTHORIZATION]);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/simulations", post(create_simulation_handler))
        .route("/api/custom_map", post(create_custom_simulation_handler))
        .route("/api/simulations/save-map", post(save_map_handler))
        .route("/api/simulations/load-map", post(load_map_handler))
        .route("/api/maps", get(list_maps_handler))
        .route("/api/maps/rename", post(rename_map_handler))
        .route("/api/maps/delete", post(delete_map_handler))
        .route("/api/simulations/:uuid/settings", get(get_simulation_settings_handler).post(update_simulation_settings_handler))
        .layer(cors)
        .with_state(shared_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
