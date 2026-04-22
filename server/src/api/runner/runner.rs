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
use crate::simulation::config::{SimulationConfig, MAX_DURATION};
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::simulation::vehicle::Vehicle;
use crate::api::runner::map_generator::{
    create_intersection_test_map, create_scheduled_simulation_seed,
};
use crate::api::runner::scheduler::{ShiftProfileInput, ShiftScheduler};
use crate::scoring::Score;

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
    scheduler: Option<Arc<Mutex<ShiftScheduler>>>,
}

impl SimulationInstance {
    pub fn new(map: crate::map::model::Map, vehicles: Vec<Vehicle>) -> Arc<Self> {
        Self::new_internal(map, vehicles, None)
    }

    pub fn new_with_shift_profiles(
        map: crate::map::model::Map,
        shift_profiles: Vec<ShiftProfileInput>,
    ) -> Result<Arc<Self>, String> {
        let seed = create_scheduled_simulation_seed(&map, shift_profiles)?;
        Ok(Self::new_internal(
            map,
            seed.vehicles,
            Some(Arc::new(Mutex::new(seed.scheduler))),
        ))
    }

    fn new_internal(
        map: crate::map::model::Map,
        vehicles: Vec<Vehicle>,
        scheduler: Option<Arc<Mutex<ShiftScheduler>>>,
    ) -> Arc<Self> {
        let token = generate_token();
        let time_step = 0.05;
        let max_steps = (MAX_DURATION / time_step).floor() as usize;

        let config = SimulationConfig {
            start_time: 0.0,
            end_time: max_steps as f32 * time_step,
            time_step,
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
            scheduler,
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

                    let (vehicles_data, traffic_lights_data, time_step, should_stop) = {
                        let mut eng = instance.engine.lock().await;
                        for _ in 0..multiplier {
                            eng.step();
                            eng.current_time += eng.config.time_step;
                        }

                        let mut scheduler_pending = false;
                        if let Some(scheduler) = &instance.scheduler {
                            let mut scheduler = scheduler.lock().await;
                            match scheduler.spawn_due_return_vehicles(
                                eng.current_time,
                                &eng.vehicles,
                                &eng.config.map,
                            ) {
                                Ok(mut spawned_vehicles) => {
                                    if !spawned_vehicles.is_empty() {
                                        eng.all_vehicles_arrived = false;
                                        eng.vehicles.append(&mut spawned_vehicles);
                                    }
                                }
                                Err(error) => {
                                    eprintln!("Failed to spawn scheduled return vehicles: {}", error);
                                }
                            }
                            scheduler_pending = scheduler.has_pending_returns();
                        }

                        let vehicles = eng.vehicles
                            .iter()
                            .map(|v| serialize_vehicle(v, &eng.config.map))
                            .collect::<Vec<_>>();
                        let tl = serialize_traffic_lights(&eng.config.map, &eng.green_links);
                        let ts = eng.config.time_step;
                        let should_stop = eng.current_time >= eng.config.end_time
                            || (eng.all_vehicles_arrived && !scheduler_pending);
                        (vehicles, tl, ts, should_stop)
                    };

                    let packet = ServerPacket::VehicleUpdate {
                        vehicles: vehicles_data,
                        traffic_lights: traffic_lights_data,
                    };
                    let _ = instance.broadcast.send(packet);

                    let elapsed = start.elapsed();
                    let step_duration = Duration::from_secs_f32(time_step / multiplier as f32);
                  
                    if should_stop {
                        let engine = instance.engine.lock().await;
                        let score:Score = engine.get_score();
                        let packet = ServerPacket::Score {
                            score : score.score,
                            total_trip_time: score.total_trip_time,
                            total_emitted_co2: score.total_emitted_co2,
                            network_length: score.network_length,
                            total_distance_traveled: score.total_distance_traveled,
                            success_rate: score.success_rate,
                        };
                        let _ = instance.broadcast.send(packet);
                        instance.controller.stop();
                        println!("Simulation finished");
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
        let map = create_intersection_test_map();
        let shift_profiles = vec![
            ShiftProfileInput {
                origin: 1,
                destination: 2,
                departure_time: 5.0,
                dwell_time: 5.0,
            },
            ShiftProfileInput {
                origin: 3,
                destination: 4,
                departure_time: 10.0,
                dwell_time: 2.0,
            },
        ];

        Self::new_with_shift_profiles(map, shift_profiles)
            .expect("default scheduled simulation should build")
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
        .layer(cors)
        .with_state(shared_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
