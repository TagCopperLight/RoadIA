use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use tokio::sync::{Mutex, broadcast};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::api::websocket::{ServerPacket, serialize_vehicle, serialize_traffic_lights};
use crate::simulation::config::SimulationConfig;
use crate::simulation::engine::{Simulation, SimulationEngine};
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
    pub initial_engine: Arc<Mutex<SimulationEngine>>,
    pub broadcast: broadcast::Sender<ServerPacket>,
    pub controller: SimulationController,
    pub active_connections: AtomicUsize,
    pub speed_multiplier: AtomicU32,
    pub file_uuid: std::sync::Mutex<Option<Uuid>>,
}

impl SimulationInstance {
    pub fn new(map: crate::map::model::Map) -> Arc<Self> {
        let vehicle_count = map.settings.vehicle_count;
        let end_time = map.settings.simulation_duration;
        let vehicles = create_random_vehicles(&map, vehicle_count);
        let token = generate_token();

        let config = SimulationConfig {
            start_time: 0.0,
            end_time,
            time_step: 0.05,
            minimum_gap: 2.0,
            score_weights: crate::simulation::config::ScoreWeights::from_settings(&map.settings),
            map,
        };

        let mut simulation = SimulationEngine::new(config, vehicles);
        for vehicle in &mut simulation.vehicles {
            vehicle.update_path(&simulation.config.map);
        }

        let initial_snapshot = simulation.clone();
        let engine = Arc::new(Mutex::new(simulation));
        let initial_engine = Arc::new(Mutex::new(initial_snapshot));
        let (broadcast, _) = broadcast::channel(100);
        let controller = SimulationController::new();

        let instance = Arc::new(Self {
            token,
            engine,
            initial_engine,
            broadcast,
            controller,
            active_connections: AtomicUsize::new(0),
            speed_multiplier: AtomicU32::new(3),
            file_uuid: std::sync::Mutex::new(None),
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

    pub fn from_file(path: &str, uuid: Uuid) -> Result<Arc<Self>, String> {
        let map = crate::map::model::Map::load(path).map_err(|e| e.to_string())?;
        let instance = Self::new(map);
        *instance.file_uuid.lock().unwrap() = Some(uuid);
        Ok(instance)
    }

    pub fn new_default() -> Arc<Self> {
        let map_path = "data/lannion.osm.pbf";
        match create_osm_map(map_path) {
            Ok(map) => {
                println!("Successfully loaded Lannion map from OSM!");
                Self::new(map)
            }
            Err(_) => {
                println!("Lannion map not found, starting with empty map.");
                Self::new(crate::map::model::Map::new())
            }
        }
    }
}

fn generate_token() -> String {
    use rand::Rng;
    let mut rng = rand::rng();
    (0..32).map(|_| format!("{:02x}", rng.random::<u8>())).collect()
}
