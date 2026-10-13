use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use tokio::sync::{Mutex, broadcast};
use tokio::time::{sleep, sleep_until, Duration, Instant};
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
                let mut next_tick = Instant::now();
                let mut next_snapshot = next_tick;
                let mut simulation_clock: Option<(f32, f64)> = None;
                loop {
                    let instance = match weak.upgrade() {
                        Some(i) => i,
                        None => break, // instance was removed, exit the loop
                    };

                    if !instance.controller.is_running() {
                        drop(instance);
                        sleep(Duration::from_millis(100)).await;
                        next_tick = Instant::now();
                        next_snapshot = next_tick;
                        continue;
                    }

                    let multiplier = instance.speed_multiplier.load(Ordering::Relaxed) as usize;

                    let (packet, time_step, finished) = {
                        let mut eng = instance.engine.lock().await;
                        // Speed changes the wall-clock cadence, not the number of steps
                        // as well: doing both applied the multiplier twice.
                        eng.step();
                        // Keep fractional time here: repeatedly adding 0.1 to an
                        // f32 clock at e.g. 08:00 otherwise accumulates timing drift.
                        let time = match simulation_clock {
                            Some((last, time)) if last == eng.current_time => time,
                            _ => eng.current_time as f64,
                        } + eng.config.time_step as f64;
                        eng.current_time = time as f32;
                        simulation_clock = Some((eng.current_time, time));
                        let finished = eng.all_vehicles_arrived || eng.current_time >= eng.config.end_time;
                        let step_interval = Duration::from_secs_f32(eng.config.time_step / multiplier as f32);
                        let snapshot_interval = step_interval.max(Duration::from_secs_f64(1.0 / 30.0));
                        // At 1x/3x send every step. At higher rates, keep physics
                        // steps small while limiting full snapshots to 30 Hz.
                        let packet = if step_interval >= snapshot_interval || Instant::now() >= next_snapshot || finished {
                            next_snapshot = Instant::now() + snapshot_interval;
                            let vehicles = eng.vehicles.iter()
                                .map(|vehicle| serialize_vehicle(vehicle, &eng.config.map))
                                .collect();
                            let traffic_lights = serialize_traffic_lights(
                                &eng.config.map, &eng.controller_green_road_ids,
                            );
                            Some(ServerPacket::VehicleUpdate {
                                vehicles,
                                traffic_lights,
                                simulation_time_s: eng.current_time,
                                playback_speed: multiplier as u32,
                                snapshot_interval_s: snapshot_interval.as_secs_f32(),
                            })
                        } else {
                            None
                        };
                        (packet, eng.config.time_step, finished)
                    };

                    if let Some(packet) = packet {
                        let _ = instance.broadcast.send(packet);
                    }

                    let step_duration = Duration::from_secs_f32(time_step / multiplier as f32);

                    if finished {
                        instance.controller.stop();
                        let _ = instance.broadcast.send(ServerPacket::SimulationFinished {});
                        println!("Simulation finished");
                    }

                    drop(instance);

                    next_tick += step_duration;
                    // Schedule from the previous deadline so timer rounding and work
                    // do not accumulate into visible pauses between snapshots.
                    if next_tick < Instant::now() {
                        next_tick = Instant::now();
                    }
                    sleep_until(next_tick).await;
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
