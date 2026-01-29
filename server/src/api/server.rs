use std::sync::Arc;
use tokio::io;
use tokio::net::TcpListener;
use axum::{Router, routing::get};
use crate::map::model::Map;
use crate::api::websocket::ws_handler;

use crate::map::intersection::{Intersection, IntersectionKind};
use crate::map::road::Road;

pub struct AppState {
    pub map: Map,
}

pub async fn run() -> io::Result<()> {
    let map = create_connected_map(100, 1000.0, 1000.0);
    
    let shared_state = Arc::new(AppState { map });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(shared_state);

    let listener = TcpListener::bind("0.0.0.0:8080").await?;
    println!("Listening on {}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    
    Ok(())
}

fn create_connected_map(num_nodes: usize, width: f32, height: f32) -> Map {
    let mut map = Map::new();
    let mut ids = 0..;

    let mut nodes = Vec::with_capacity(num_nodes);

    // 1. Create random nodes
    for _ in 0..num_nodes {
        let id = ids.next().unwrap();
        let kind = match rand::random_range(0..5) {
            0 => IntersectionKind::Habitation,
            1 => IntersectionKind::Workplace,
            _ => IntersectionKind::Intersection,
        };

        let node_idx = map.add_intersection(Intersection {
            id,
            kind,
            name: format!("node_{}", id),
            x: rand::random_range(0.0..width),
            y: rand::random_range(0.0..height),
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
        let speed_limit = rand::random_range(13..33) as u8;
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
                let speed_limit = rand::random_range(13..33) as u8;
                map.add_two_way_road(u, v, Road::new(road_id, 1, speed_limit, dist, false, false));
            }
        }
    }

    map
}

fn run_simulation(map: &Map) {
    
}