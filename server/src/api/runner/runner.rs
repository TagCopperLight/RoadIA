use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::io;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};
use axum::{Router, routing::get};

use crate::map::model::Map;
use crate::api::websocket::{ws_handler, ServerPacket, WebSocketService, serialize_vehicle};
use crate::simulation::config::SimulationConfig;
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::api::runner::map_generator::{create_intersection_test_map, create_random_vehicles};

#[derive(Clone)]
pub struct SimulationController {
    running: Arc<AtomicBool>,
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
    pub map: Map,
    pub websocket_service: Arc<WebSocketService>,
    pub simulation: SimulationController,
}

pub async fn run() -> io::Result<()> {
    // let map = create_connected_map(200, 1500.0, 1500.0);
    let map = create_intersection_test_map();
    let vehicles = create_random_vehicles(&map, 500);
    
    let config = SimulationConfig {
        start_time: 0.0,
        end_time: f32::MAX,
        time_step: 0.1,
        minimum_gap: 2.0,
        air_density: 1.225, // en Mg/m^3
        gravity_coefficient: 9.81,
        time_weight : 0.5,
        succes_weight: 0.3,
        pollution_weight: 0.2,
        map: map.clone(),
    };

    let mut simulation = SimulationEngine::new(config, vehicles);
    
    // Initialize vehicle paths
    for vehicle in &mut simulation.vehicles {
        vehicle.update_path(&simulation.config.map);
    }
    
    let websocket_service = Arc::new(WebSocketService::new());
    let simulation_controller = SimulationController::new();
    
    // Spawn simulation loop
    let ws_service = websocket_service.clone();
    let sim_map = map.clone();
    let sim_controller = simulation_controller.clone();

    tokio::spawn(async move {
        loop {
            if !sim_controller.is_running() {
                sleep(Duration::from_millis(100)).await;
                continue;
            }

            let start = tokio::time::Instant::now();
            simulation.step();
            simulation.current_time += simulation.config.time_step;
            
            // Broadcast vehicle updates
            let vehicles_data = simulation.vehicles.iter().map(|v| {
                serialize_vehicle(v, &sim_map)
            }).collect();
            
            let packet = ServerPacket::VehicleUpdate { vehicles: vehicles_data };
            ws_service.send(packet);

            let elapsed = start.elapsed();
            if elapsed < Duration::from_millis(10) {
                 sleep(Duration::from_millis(10) - elapsed).await;
            }
        }
    });

    let shared_state = Arc::new(AppState { map, websocket_service, simulation: simulation_controller });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(shared_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    
    Ok(())
}