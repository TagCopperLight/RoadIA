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
use crate::api::runner::map_generator::{create_lane_restriction_map, create_vehicle_for_lane_restriction_map};
use petgraph::Direction;
use petgraph::visit::EdgeRef;

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
    // Use the lane-restriction test map and its single test vehicle
    let map = create_lane_restriction_map();
    let mut vehicles = create_vehicle_for_lane_restriction_map(&map);

    // Debug: print edges, lanes and their links to investigate routing
    println!("Map edges and links:");
    for e in map.graph.edge_indices() {
        if let Some((a, b)) = map.graph.edge_endpoints(e) {
            let road = &map.graph[e];
            println!("  road id={} from={} to={} lanes={} length={}", road.id, map.graph[a].id, map.graph[b].id, road.lanes.len(), road.length);
            for (li, lane) in road.lanes.iter().enumerate() {
                let link_ids: Vec<String> = lane.links.iter().map(|l| format!("{}->via_il{}", l.destination_road_id, l.via_internal_lane_id)).collect();
                println!("    lane {} (id={}): links: {:?}", li, lane.id, link_ids);
            }
        }
    }

    // Print intersections and internal lanes for debugging (inspect junction B)
    println!("Map nodes (intersections):");
    for n in map.graph.node_indices() {
        let node = &map.graph[n];
        println!("  node id={} kind={:?} coords=({:.1},{:.1}) internal_lanes={}", node.id, node.kind, node.center_coordinates.x, node.center_coordinates.y, node.internal_lanes.len());
        for il in &node.internal_lanes {
            println!("    internal_lane id={} from_lane_id={} to_lane_id={} length={:.1} entry={:?} exit={:?}", il.id, il.from_lane_id, il.to_lane_id, il.length, il.entry, il.exit);
        }

        // List incoming/outgoing roads and their lane->link mapping touching this node
        let mut incoming: Vec<_> = map.graph.edges_directed(n, Direction::Incoming).collect();
        let mut outgoing: Vec<_> = map.graph.edges_directed(n, Direction::Outgoing).collect();
        println!("    incoming edges: {} outgoing edges: {}", incoming.len(), outgoing.len());
        for e in incoming.iter().chain(outgoing.iter()) {
            let edge_idx = e.id();
            let road = &map.graph[edge_idx];
            println!("      road id={} lanes={}", road.id, road.lanes.len());
            for lane in &road.lanes {
                for link in &lane.links {
                    println!("        lane id={} -> link id={} dest_road={} via_il={}", lane.id, link.id, link.destination_road_id, link.via_internal_lane_id);
                }
            }
        }
    }

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
