use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use tokio::io;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use axum::{Router, routing::{get, post}, extract::State, Json};
use uuid::Uuid;
use axum::http::{HeaderValue, Method, header::CONTENT_TYPE};
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::api::websocket::{ws_handler, ServerPacket, serialize_vehicle, serialize_traffic_lights};
use crate::simulation::config::SimulationConfig;
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::simulation::vehicle::Vehicle;
use crate::api::runner::map_generator::{create_random_vehicles, create_osm_map};

#[derive(Clone)]
pub struct SimulationController {
    running: Arc<AtomicBool>,
}

impl Default for SimulationController {
    fn default() -> Self {
        Self::new()
    }
}

impl SimulationController {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start(&self) {
        self.running.store(true, Ordering::SeqCst);
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

pub struct SimulationInstance {
    pub token: String,
    pub engine: Arc<Mutex<SimulationEngine>>,
    pub broadcast: broadcast::Sender<ServerPacket>,
    pub controller: SimulationController,
    pub active_connections: AtomicUsize,
    pub speed_multiplier: AtomicU32,
}

impl SimulationInstance {
    pub fn new(map: crate::map::model::Map, vehicles: Vec<Vehicle>) -> Arc<Self> {
        let token = generate_token();

        let config = SimulationConfig {
            start_time: 0.0,
            end_time: 600.0,
            time_step: 0.05,
            minimum_gap: 2.0,
            map,
        };

        let mut simulation = SimulationEngine::new(config, vehicles);
        for vehicle in &mut simulation.vehicles {
            vehicle.update_path(&simulation.config.map);
        }

        let engine = Arc::new(Mutex::new(simulation));
        let (broadcast, _) = broadcast::channel(100);
        let controller = SimulationController::new();

        let instance = Arc::new(Self {
            token,
            engine,
            broadcast,
            controller,
            active_connections: AtomicUsize::new(0),
            speed_multiplier: AtomicU32::new(3),
        });

        tokio::spawn({
            let weak = Arc::downgrade(&instance);
            async move {
                loop {
                    let instance = match weak.upgrade() {
                        Some(i) => i,
                        None => break, // instance was removed, exit the loop
                    };

                    if !instance.controller.is_running() {
                        drop(instance);
                        sleep(Duration::from_millis(100)).await;
                        continue;
                    }

                    let start = tokio::time::Instant::now();
                    let multiplier = instance.speed_multiplier.load(Ordering::Relaxed) as usize;

                    let (vehicles_data, traffic_lights_data, time_step) = {
                        let mut eng = instance.engine.lock().await;
                        for _ in 0..multiplier {
                            eng.step();
                            eng.current_time += eng.config.time_step;
                        }
                        let vehicles = eng.vehicles
                            .iter()
                            .map(|v| serialize_vehicle(v, &eng.config.map))
                            .collect::<Vec<_>>();
                        let tl = serialize_traffic_lights(&eng.config.map, &eng.green_links);
                        let ts = eng.config.time_step;
                        (vehicles, tl, ts)
                    };

                    let packet = ServerPacket::VehicleUpdate {
                        vehicles: vehicles_data,
                        traffic_lights: traffic_lights_data,
                    };
                    let _ = instance.broadcast.send(packet);

                    let elapsed = start.elapsed();
                    let step_duration = Duration::from_secs_f32(time_step / multiplier as f32);
                  
                    {
                        let engine = instance.engine.lock().await;
                        if engine.all_vehicles_arrived || engine.current_time >= engine.config.end_time {
                            instance.controller.stop();
                            let _ = instance.broadcast.send(ServerPacket::SimulationFinished {});
                            println!("Simulation finished");
                        }
                    }
                  
                    drop(instance);
                    
                    if elapsed < step_duration {
                        sleep(step_duration - elapsed).await;
                    }                  
                }
            }
        });

        instance
    }

    pub fn from_file(path: &str) -> Result<Arc<Self>, String> {
        let map = crate::map::model::Map::load(path).map_err(|e| e.to_string())?;
        let vehicles = create_random_vehicles(&map, 500);
        Ok(Self::new(map, vehicles))
    }

    pub fn new_default() -> Arc<Self> {
        // let map = create_connected_map(200, 1500.0, 1500.0);
        // let map = create_traffic_light_test_map();

        let map_path = "data/lannion.osm.pbf";
        match create_osm_map(map_path) {
            Ok(map) => {
                println!("Successfully loaded Lannion map from OSM!");
                let vehicles = create_random_vehicles(&map, 500);
                Self::new(map, vehicles)
            }
            Err(e) => {
                println!("Failed to load Lannion map: {:?}", e);
                panic!("Failed to load Lannion map: {:?}", e);
            }
        }
    }
}

fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..32).map(|_| format!("{:02x}", rng.random::<u8>())).collect()
}

pub struct AppState {
    pub simulations: Arc<RwLock<HashMap<Uuid, Arc<SimulationInstance>>>>,
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
    filename: String,
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

    let engine = instance.engine.lock().await;
    let map = &engine.config.map;

    let path = format!("data/{}", payload.filename);
    if let Err(e) = map.save(&path) {
        println!("Failed to save map: {:?}", e);
        return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(Json(serde_json::json!({ "status": "success", "path": path })))
}

#[derive(serde::Deserialize)]
struct LoadMapRequest {
    filename: String,
}

async fn load_map_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<LoadMapRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let path = format!("data/{}", payload.filename);
    match SimulationInstance::from_file(&path) {
        Ok(instance) => {
            let uuid = Uuid::new_v4();
            let token = instance.token.clone();
            state.simulations.write().await.insert(uuid, instance);
            Ok(Json(serde_json::json!({ "uuid": uuid, "token": token })))
        }
        Err(e) => {
            println!("Failed to load map from {}: {:?}", path, e);
            Err(axum::http::StatusCode::NOT_FOUND)
        }
    }
}

#[derive(serde::Deserialize)]
struct RenameMapRequest {
    old_filename: String,
    new_filename: String,
}

async fn rename_map_handler(
    Json(payload): Json<RenameMapRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let old_path = format!("data/{}", payload.old_filename);
    let new_path = format!("data/{}", payload.new_filename);
    std::fs::rename(&old_path, &new_path).map_err(|e| {
        println!("Failed to rename map: {:?}", e);
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    })?;
    Ok(Json(serde_json::json!({ "status": "success" })))
}

async fn list_maps_handler() -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    let data_dir = "data";
    let mut maps = Vec::new();

    if let Ok(entries) = std::fs::read_dir(data_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    maps.push(filename.to_string());
                }
            }
        }
    }

    Ok(Json(serde_json::json!({ "maps": maps })))
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
        .route("/api/simulations/save-map", post(save_map_handler))
        .route("/api/simulations/load-map", post(load_map_handler))
        .route("/api/maps", get(list_maps_handler))
        .route("/api/maps/rename", post(rename_map_handler))
        .layer(cors)
        .with_state(shared_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
