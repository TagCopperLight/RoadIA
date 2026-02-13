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

use crate::map::intersection::{Intersection, IntersectionKind};
use crate::map::road::Road;

pub struct AppState {
    pub map: Map,
    pub websocket_service: Arc<WebSocketService>,
}

pub async fn run() -> io::Result<()> {
    let map = create_connected_map(200, 1500.0, 1500.0);
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

fn create_connected_map(num_nodes: usize, width: f32, height: f32) -> Map {
    let mut map = Map::new();
    let mut ids = 0..;

    let mut nodes = Vec::with_capacity(num_nodes);

    // 1. Create random nodes
    for i in 0..num_nodes {
        let id = ids.next().unwrap();
        // Ensure at least one Habitation and one Workplace
        let kind = if i == 0 {
            IntersectionKind::Habitation
        } else if i == 1 {
            IntersectionKind::Workplace
        } else {
            match rand::random_range(0..10) {
                0 => IntersectionKind::Habitation,
                1 => IntersectionKind::Workplace,
                _ => IntersectionKind::Intersection,
            }
        };

        let node_idx = map.add_intersection(Intersection {
            id,
            kind,
            name: format!("node_{}", id),
            x: rand::random_range(0.0..width),
            y: rand::random_range(0.0..height),
            occupied: false
        });
        nodes.push(node_idx);
    }

    // 2. Build MST to ensure connectivity
    // Simple Prim's like approach:
    // Start with first node in connected set.
    // Iteratively add the closest node not in the set to the set.
    let mut connected_indices = vec![0];
    let mut available_indices: Vec<usize> = (1..num_nodes).collect();
    let mut road_ids = 0..;

    while !available_indices.is_empty() {
        let mut min_dist = f32::MAX;
        let mut best_u = 0;
        let mut best_v_idx_in_available = 0;

        for &u_idx in &connected_indices {
            for (i, &v_idx) in available_indices.iter().enumerate() {
                let u = nodes[u_idx];
                let v = nodes[v_idx];
                let dist = map.intersections_euclidean_distance(u, v);

                if dist < min_dist {
                    min_dist = dist;
                    best_u = u_idx;
                    best_v_idx_in_available = i;
                }
            }
        }

        let best_v = available_indices.remove(best_v_idx_in_available);
        connected_indices.push(best_v);

        // Add edge
        let u = nodes[best_u];
        let v = nodes[best_v];

        let road_id = road_ids.next().unwrap();
        let speed_limit = rand::random_range(13..33) as f32;
        map.add_two_way_road(
            u,
            v,
            Road::new(road_id, 1, speed_limit, min_dist, false, false),
        );
    }

    // 3. Add extra edges for cycles (connect to k nearest neighbors)
    let extra_connections = 2;

    for (i, &u) in nodes.iter().enumerate() {
        let mut neighbors: Vec<(usize, f32)> = nodes
            .iter()
            .enumerate()
            .filter(|&(j, _)| i != j)
            .map(|(j, &v)| {
                let dist = map.intersections_euclidean_distance(u, v);
                (j, dist)
            })
            .collect();

        neighbors.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        for k in 0..extra_connections.min(neighbors.len()) {
            let (v_idx, dist) = neighbors[k];
            let v = nodes[v_idx];

            // Only add if not strictly existing?
            // map.graph checks for existing index but let's check edge existence to avoid duplicates if possible
            if map.graph.find_edge(u, v).is_none() {
                let road_id = road_ids.next().unwrap();
                let speed_limit = rand::random_range(13..33) as f32;
                map.add_two_way_road(u, v, Road::new(road_id, 1, speed_limit, dist, false, false));
            }
        }
    }

    map
}