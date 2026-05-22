use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use tokio::sync::{Mutex, broadcast};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

use crate::api::websocket::{ServerPacket, serialize_vehicle, serialize_traffic_lights};
use crate::simulation::config::SimulationConfig;
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::api::runner::map_generator::{create_random_commutes, create_osm_map};


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
    pub score_progress_broadcast: broadcast::Sender<ServerPacket>,
    pub controller: SimulationController,
    pub active_connections: AtomicUsize,
    pub speed_multiplier: AtomicU32,
    pub file_uuid: std::sync::Mutex<Option<Uuid>>,
}

impl SimulationInstance {
    pub fn new(map: crate::map::model::Map) -> Arc<Self> {
        let commute_plan_count = map.settings.vehicle_count;
        let end_time = if map.settings.use_day_night_cycle {
            86_400.0f32
        } else {
            map.settings.simulation_start_time + 3_600.0f32
        };
        println!(
            "Simulation settings: vehicle_count={}, simulation_start_time={}, time_step={}",
            map.settings.vehicle_count,
            map.settings.simulation_start_time,
            map.settings.time_step,
        );
        let generated = create_random_commutes(&map, commute_plan_count);
        let token = generate_token();

        let config = SimulationConfig {
            start_time: map.settings.simulation_start_time,
            end_time,
            time_step: map.settings.time_step,
            minimum_gap: 2.0,
            score_weights: crate::simulation::config::ScoreWeights::from_settings(&map.settings),
            map,
        };

        let mut simulation = SimulationEngine::new_with_commutes(config, generated.vehicles, generated.commute_plans);

        // Recreate bus vehicles from saved bus lines in the map
        simulation.next_bus_line_id = simulation.config.map.next_bus_line_id;
        for bl in simulation.config.map.bus_lines.clone() {
            simulation.add_bus_from_saved(&bl);
        }

        let initial_snapshot = simulation.clone();
        let engine = Arc::new(Mutex::new(simulation));
        let initial_engine = Arc::new(Mutex::new(initial_snapshot));
        let (broadcast, _) = broadcast::channel(50);
        let (score_progress_broadcast, _) = broadcast::channel(100);
        let controller = SimulationController::new();

        let instance = Arc::new(Self {
            token,
            engine,
            initial_engine,
            broadcast,
            score_progress_broadcast,
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

                    let (map_snapshot, vehicles_snapshot, controller_green_road_ids, simulation_time_s, time_step, finished) = {
                        let mut eng = instance.engine.lock().await;
                        for _ in 0..multiplier {
                            eng.step();
                            eng.current_time += eng.config.time_step;
                        }
                        let map_snapshot = eng.config.map.clone();
                        let vehicles_snapshot = eng.vehicles.clone();
                        let controller_green_road_ids = eng.controller_green_road_ids.clone();
                        let finished = eng.all_vehicles_arrived || eng.current_time >= eng.config.end_time;
                        let ts = eng.config.time_step;
                        (map_snapshot, vehicles_snapshot, controller_green_road_ids, eng.current_time, ts, finished)
                    };

                    let mut vehicles_data = Vec::with_capacity(vehicles_snapshot.len());
                    for vehicle in &vehicles_snapshot {
                        vehicles_data.push(serialize_vehicle(vehicle, &map_snapshot));
                    }
                    let traffic_lights_data = serialize_traffic_lights(&map_snapshot, &controller_green_road_ids);

                    let packet = ServerPacket::VehicleUpdate {
                        vehicles: vehicles_data,
                        traffic_lights: traffic_lights_data,
                        simulation_time_s,
                    };
                    let _ = instance.broadcast.send(packet);

                    let elapsed = start.elapsed();
                    let step_duration = Duration::from_secs_f32(time_step / multiplier as f32)
                        .max(Duration::from_millis(50));

                    if finished {
                        instance.controller.stop();
                        let _ = instance.broadcast.send(ServerPacket::SimulationFinished {});
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
                Self::new(map)
            }
            Err(_) => {
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
