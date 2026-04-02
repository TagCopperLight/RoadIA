use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io;
use tokio::net::TcpListener;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use axum::{Router, routing::get};

use crate::api::websocket::{ws_handler, ServerPacket, WebSocketService, serialize_vehicle};
use crate::simulation::config::SimulationConfig;
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::api::runner::map_generator::{create_connected_map, create_random_vehicles};
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

pub struct AppState {
    pub engine: Arc<Mutex<SimulationEngine>>,
    pub websocket_service: Arc<WebSocketService>,
    pub simulation: SimulationController,
}

pub async fn run() -> io::Result<()> {
    let map = create_connected_map(200, 1500.0, 1500.0);
    let vehicles = create_random_vehicles(&map, 50);

    let config = SimulationConfig {
        start_time: 0.0,
        end_time: f32::MAX,
        time_step: 0.05,
        minimum_gap: 2.0,
        map,
    };

    let mut simulation = SimulationEngine::new(config, vehicles);
    
    // Initialize vehicle paths
    for vehicle in &mut simulation.vehicles {
        vehicle.update_path(&simulation.config.map);
    }

    let engine = Arc::new(Mutex::new(simulation));
    let websocket_service = Arc::new(WebSocketService::new());
    let controller = SimulationController::new();

    // Spawn simulation loop
    tokio::spawn({
        let engine = Arc::clone(&engine);
        let websocket_service = websocket_service.clone();
        let controller = controller.clone();

        async move {
            loop {
                if !controller.is_running() {
                    sleep(Duration::from_millis(100)).await;
                    continue;
                }

                let start = tokio::time::Instant::now();

                let vehicles_data = {
                    let mut engine = engine.lock().await;
                    engine.step();
                    engine.current_time += engine.config.time_step;
                    engine.vehicles
                        .iter()
                        .map(|v| serialize_vehicle(v, &engine.config.map))
                        .collect::<Vec<_>>()
                };

                let packet = ServerPacket::VehicleUpdate { vehicles: vehicles_data };
                websocket_service.send(packet);

                let elapsed = start.elapsed();
                if elapsed < Duration::from_millis(10) {
                    sleep(Duration::from_millis(10) - elapsed).await;
                }

                let engine = engine.lock().await;
                if engine.all_vehicles_arrived {
                    // show score here
                    let score:Score = engine.get_score();
                    let packet = ServerPacket::Score {
                        score : score.score,
                        total_trip_time: score.total_trip_time,
                        total_emitted_co2: score.total_emitted_co2,
                        network_length: score.network_length,
                        success_rate: score.success_rate,
                    };
                    websocket_service.send(packet);
                    controller.stop();
                    println!("Simulation finished");
                }
            }
        }
    });

    let shared_state = Arc::new(AppState {
        engine,
        websocket_service,
        simulation: controller,
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(shared_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;

    Ok(())
}
