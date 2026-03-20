use petgraph::graph::NodeIndex;

use crate::map::intersection::{IntersectionKind, IntersectionRules, IntersectionType};
use crate::map::model::Map;
use crate::simulation::vehicle::{Vehicle, VehicleSpec, VehicleKind, TripRequest};


pub fn create_random_vehicles(map: &Map, count: usize) -> Vec<Vehicle> {
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
            length: 10.0,
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

pub fn create_connected_map(num_nodes: usize, width: f32, height: f32) -> Map {
    let mut map = Map::new();

    let mut nodes = Vec::with_capacity(num_nodes);
    const MIN_NODE_SPACING: f32 = 30.0;
    let mut positions: Vec<(f32, f32)> = Vec::with_capacity(num_nodes);

    // 1. Create random nodes
    let mut i = 0;
    while i < num_nodes {
        let x = rand::random_range(0.0..width);
        let y = rand::random_range(0.0..height);

        if positions.iter().any(|&(px, py)| (x - px).hypot(y - py) < MIN_NODE_SPACING) {
            continue;
        }

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

        let node_idx = map.add_intersection(
            kind,
            format!("node_{}", i),
            x, y,
            IntersectionType::Priority,
        );
        nodes.push(node_idx);
        positions.push((x, y));
        i += 1;
    }

    // 2. Build MST to ensure connectivity
    // Simple Prim's like approach:
    // Start with first node in connected set.
    // Iteratively add the closest node not in the set to the set.
    let mut connected_indices = vec![0];
    let mut available_indices: Vec<usize> = (1..num_nodes).collect();

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

        let u = nodes[best_u];
        let v = nodes[best_v];
        let speed_limit = rand::random_range(13..33) as f32;
        map.add_two_way_road(u, v, 1, speed_limit, min_dist, false, false);
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

            if map.graph.find_edge(u, v).is_none() {
                let speed_limit = rand::random_range(13..33) as f32;
                map.add_two_way_road(u, v, 1, speed_limit, dist, false, false);
            }
        }
    }

    map
}

pub fn create_one_intersection_congestion_map() -> Map {
    let mut map = Map::new();

    let h1 = map.add_intersection(IntersectionKind::Habitation, "habitation 1".into(), 0.0, 0.0, IntersectionType::Priority);
    let h2 = map.add_intersection(IntersectionKind::Habitation, "habitation 2".into(), 0.0, 100.0, IntersectionType::Priority);
    let i1 = map.add_intersection(IntersectionKind::Intersection, "intersection 1".into(), 50.0, 50.0, IntersectionType::Priority);
    let w1 = map.add_intersection(IntersectionKind::Workplace, "workplace 1".into(), 950.0, 50.0, IntersectionType::Priority);

    map.add_two_way_road(h1, i1, 1, 40.0, 70.0, false, false);
    map.add_two_way_road(h2, i1, 1, 40.0, 70.0, false, false);
    map.add_two_way_road(i1, w1, 1, 40.0, 950.0, false, false);

    map
}

pub fn create_intersection_test_map() -> Map {
    let mut map = Map::new();

    let center = map.add_intersection(IntersectionKind::Intersection, "Center".into(), 500.0, 500.0,  IntersectionType::Priority);
    let north  = map.add_intersection(IntersectionKind::Habitation,   "North".into(),  500.0, 0.0,    IntersectionType::Priority);
    let south  = map.add_intersection(IntersectionKind::Workplace,    "South".into(),  500.0, 1000.0, IntersectionType::Priority);
    let east   = map.add_intersection(IntersectionKind::Habitation,   "East".into(),   1000.0, 500.0, IntersectionType::Priority);
    let west   = map.add_intersection(IntersectionKind::Workplace,    "West".into(),   0.0,    500.0, IntersectionType::Priority);

    // N -> C and S -> C are Priority (default)
    map.add_two_way_road(north, center, 1, 40.0, 500.0, false, false);
    map.add_two_way_road(south, center, 1, 40.0, 500.0, false, false);

    // E -> C and W -> C are Yield — capture the inbound road IDs to override rules
    let (east_road_id, _) = map.add_two_way_road(east, center, 1, 40.0, 500.0, false, false);
    let (west_road_id, _) = map.add_two_way_road(west, center, 1, 40.0, 500.0, false, false);

    if let Some(center_node) = map.graph.node_weight_mut(center) {
        center_node.set_rule(east_road_id, IntersectionRules::Yield);
        center_node.set_rule(west_road_id, IntersectionRules::Yield);
    }

    map
}
