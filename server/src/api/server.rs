use std::sync::Arc;
use tokio::io;
use tokio::net::TcpListener;
use tokio::time::{sleep, Duration};
use axum::{Router, routing::get};
use serde_json::json;

use crate::map::model::Map;
use crate::api::websocket::{ws_handler, ServerPacket, WebSocketService};
use crate::simulation::config::SimulationConfig;
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::simulation::vehicle::{Vehicle, VehicleSpec, VehicleKind, TripRequest, VehicleState};
use petgraph::graph::NodeIndex;

use crate::map::intersection::{IntersectionKind};

use crate::api::map_generator::{create_connected_map, create_one_road_map, create_one_intersection_congestion_map, create_bone_map, create_mistakes_map};

pub struct AppState {
    pub map: Map,
    pub websocket_service: Arc<WebSocketService>,
}

pub async fn run() -> io::Result<()> {
    //let map = create_connected_map(200, 1500.0, 1500.0);
    let map = create_mistakes_map();
    let vehicles = create_random_vehicles(&map, 100);
    
    let config = SimulationConfig {
        start_time: 0.0,
        end_time: f32::MAX,
        time_step: 0.1,
        minimum_gap: 2.0,
        path_mistake_rate: 0.1,
        map: map.clone(),
    };

    let mut simulation = SimulationEngine::new(config, vehicles);
    
    // Initialize vehicle paths
    for vehicle in &mut simulation.vehicles {
        vehicle.init_path(&simulation.config.map);
    }
    
    let websocket_service = Arc::new(WebSocketService::new());
    
    // Spawn simulation loop
    let ws_service = websocket_service.clone();
    let sim_map = map.clone();
    tokio::spawn(async move {
        loop {
            let start = tokio::time::Instant::now();
            simulation.step();
            
            // Broadcast vehicle updates
            let vehicles_data: Vec<_> = simulation.vehicles.iter().map(|v| {
                let coords = v.get_coordinates(&sim_map);
                json!({
                    "id": v.id,
                    "x": coords.x,
                    "y": coords.y,
                    "kind": match v.spec.kind {
                         VehicleKind::Car => "Car",
                         VehicleKind::Bus => "Bus",
                    },
                    "state": match v.state {
                        VehicleState::WaitingToDepart => "Waiting",
                        VehicleState::OnRoad => "Moving",
                        VehicleState::AtIntersection => "Intersection",
                        VehicleState::Arrived => "Arrived",
                    }
                })
            }).collect();
            
            let packet = ServerPacket::VehicleUpdate { vehicles: vehicles_data };
            ws_service.send(packet);

            let elapsed = start.elapsed();
            if elapsed < Duration::from_millis(30) {
                 sleep(Duration::from_millis(30) - elapsed).await;
            }
        }
    });

    let shared_state = Arc::new(AppState { map, websocket_service });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(shared_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    
    Ok(())
}

fn create_random_vehicles(map: &Map, count: usize) -> Vec<Vehicle> {
    let mut vehicles = Vec::new();
    let mut ids = 0..;
    
    let nodes: Vec<NodeIndex> = map.graph.node_indices().collect();
    if nodes.is_empty() {
        return vehicles;
    }

    let habitations: Vec<NodeIndex> = nodes.iter()
        .filter(|&&n| matches!(map.graph[n].kind, IntersectionKind::Habitation))
        .copied()
        .collect();

    let workplaces: Vec<NodeIndex> = nodes.iter()
        .filter(|&&n| matches!(map.graph[n].kind, IntersectionKind::Workplace))
        .copied()
        .collect();

    if habitations.is_empty() || workplaces.is_empty() {
        println!("Warning: Cannot create vehicles, missing Habitation or Workplace nodes");
        return vehicles;
    }

    for _ in 0..count {
        let origin = habitations[rand::random_range(0..habitations.len())];
        let destination = workplaces[rand::random_range(0..workplaces.len())];

        let spec = VehicleSpec {
            kind: VehicleKind::Car,
            max_speed: 40.0, // m/s
            max_acceleration: 4.0,
            comfortable_deceleration: 3.0,
            reaction_time: 1.0,
            length: 4.5,
        };

        let trip = TripRequest {
            origin,
            destination,
            departure_time: 0,
            return_time: None,
        };

        vehicles.push(Vehicle::new(ids.next().unwrap(), spec, trip));
    }
    
    vehicles
}