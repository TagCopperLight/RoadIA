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
    let uuid = Uuid::new_v4();
    let tmp_osm_path = format!("data/tmp/{}.osm", uuid);
    let tmp_pbf_path = format!("data/tmp/{}.osm.pbf", uuid);

    let overpass_query = format!(
        "[out:xml][timeout:25][maxsize:1073741824];(way[highway]({min_lat},{min_lon},{max_lat},{max_lon});>;);out body;",
        min_lat = payload.min_lat,
        min_lon = payload.min_lon,
        max_lat = payload.max_lat,
        max_lon = payload.max_lon
    );

    // Download map via reqwest
    let client = reqwest::Client::builder()
        .user_agent("RoadIA/0.1.0")
        .build()
        .unwrap();
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

    std::fs::write(&tmp_osm_path, map_data).map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to write temporary osm file".to_string()))?;

    // Convert with osmium
    let osmium_status = std::process::Command::new("osmium")
        .args(&["cat", &tmp_osm_path, "-o", &tmp_pbf_path])
        .status()
        .map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to execute osmium".to_string()))?;

    if !osmium_status.success() {
        return Err((axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Osmium failed to convert the file".to_string()));
    }

    // Load newly created map
    let map = create_osm_map(&tmp_pbf_path).map_err(|_| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, "Failed to parse custom mapped region".to_string()))?;
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
