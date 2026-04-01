use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, RwLock};
use tokio::sync::broadcast;
use tokio::time::{sleep, Duration};
use axum::{Router, routing::{get, post}, extract::State, Json};
use uuid::Uuid;
use tower_http::cors::CorsLayer;

use crate::api::websocket::{ws_handler, ServerPacket, serialize_vehicle, serialize_traffic_lights};
use crate::simulation::config::SimulationConfig;
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::simulation::vehicle::Vehicle;
use crate::api::runner::map_generator::{create_roundabout_test_map, create_random_vehicles};

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
}

impl SimulationInstance {
    pub fn new(map: crate::map::model::Map, vehicles: Vec<Vehicle>) -> Arc<Self> {
        let token = generate_token();

        let config = SimulationConfig {
            start_time: 0.0,
            end_time: f32::MAX,
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

        let instance = Arc::new(Self { token, engine, broadcast, controller });

        tokio::spawn({
            let instance = Arc::clone(&instance);
            async move {
                loop {
                    if !instance.controller.is_running() {
                        sleep(Duration::from_millis(100)).await;
                        continue;
                    }

                    let start = tokio::time::Instant::now();

                    let (vehicles_data, traffic_lights_data) = {
                        let mut eng = instance.engine.lock().await;
                        eng.step();
                        eng.current_time += eng.config.time_step;
                        let vehicles = eng.vehicles
                            .iter()
                            .map(|v| serialize_vehicle(v, &eng.config.map))
                            .collect::<Vec<_>>();
                        let tl = serialize_traffic_lights(&eng.config.map, &eng.green_links);
                        (vehicles, tl)
                    };

                    let packet = ServerPacket::VehicleUpdate {
                        vehicles: vehicles_data,
                        traffic_lights: traffic_lights_data,
                    };
                    let _ = instance.broadcast.send(packet);

                    let elapsed = start.elapsed();
                    if elapsed < Duration::from_millis(10) {
                        sleep(Duration::from_millis(10) - elapsed).await;
                    }
                }
            }
        });

        instance
    }

    pub fn new_default() -> Arc<Self> {
        // let map = create_connected_map(200, 1500.0, 1500.0);
        let map = create_roundabout_test_map();
        let vehicles = create_random_vehicles(&map, 50);
        Self::new(map, vehicles)
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

pub async fn run() -> io::Result<()> {
    let shared_state = Arc::new(AppState {
        simulations: Arc::new(RwLock::new(HashMap::new())),
    });

    let cors = CorsLayer::permissive();

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/api/simulations", post(create_simulation_handler))
        .layer(cors)
        .with_state(shared_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
