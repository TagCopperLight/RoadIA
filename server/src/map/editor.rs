use petgraph::visit::EdgeRef;
use petgraph::Direction;

use crate::map::intersection::IntersectionKind;
use crate::map::model::Map;
use crate::map::road::LinkType;
use crate::map::roundabout::RoundaboutHandle;
use crate::map::traffic_light::{SignalPhase, TrafficLightController, TrafficLightControllerHandle};
use crate::simulation::config::MAX_SPEED;

pub fn add_node(map: &mut Map, x: f32, y: f32, kind: IntersectionKind) -> u32 {
    map.add_intersection(kind, x, y)
}

pub fn delete_node(map: &mut Map, id: u32) -> Result<(), String> {
    let idx = map
        .node_index_map
        .get(&id)
        .copied()
        .ok_or_else(|| format!("Node {} not found", id))?;

    map.node_index_map.remove(&id);
    map.graph.remove_node(idx);

    if let Some(swapped) = map.graph.node_weight(idx) {
        let swapped_id = swapped.id;
        map.node_index_map.insert(swapped_id, idx);
    }

    Ok(())
}

pub fn move_node(map: &mut Map, id: u32, x: f32, y: f32) -> Result<(), String> {
    let idx = map
        .node_index_map
        .get(&id)
        .copied()
        .ok_or_else(|| format!("Node {} not found", id))?;

    map.graph[idx].center_coordinates.x = x;
    map.graph[idx].center_coordinates.y = y;

    // Recalculate lengths of all connected edges.
    let edge_indices: Vec<_> = map
        .graph
        .edges_directed(idx, Direction::Incoming)
        .chain(map.graph.edges_directed(idx, Direction::Outgoing))
        .map(|e| e.id())
        .collect();

    for edge_idx in edge_indices {
        let (a, b) = map.graph.edge_endpoints(edge_idx).unwrap();
        let ax = map.graph[a].center_coordinates.x;
        let ay = map.graph[a].center_coordinates.y;
        let bx = map.graph[b].center_coordinates.x;
        let by = map.graph[b].center_coordinates.y;
        let dx = bx - ax;
        let dy = by - ay;
        let ra = map.graph[a].radius;
        let rb = map.graph[b].radius;
        map.graph[edge_idx].length = ((dx * dx + dy * dy).sqrt() - ra - rb).max(1.0);
    }

    Ok(())
}

pub fn update_node(
    map: &mut Map,
    id: u32,
    kind: IntersectionKind,
) -> Result<(), String> {
    let idx = map
        .node_index_map
        .get(&id)
        .copied()
        .ok_or_else(|| format!("Node {} not found", id))?;

    map.graph[idx].kind = kind;

    Ok(())
}

pub fn add_road(
    map: &mut Map,
    from_id: u32,
    to_id: u32,
    lane_count: u8,
    speed_limit: f32,
) -> Result<u32, String> {
    let from_idx = map
        .node_index_map
        .get(&from_id)
        .copied()
        .ok_or_else(|| format!("Node {} not found", from_id))?;
    let to_idx = map
        .node_index_map
        .get(&to_id)
        .copied()
        .ok_or_else(|| format!("Node {} not found", to_id))?;

    if map.graph.find_edge(from_idx, to_idx).is_some() {
        return Err(format!("Road from {} to {} already exists", from_id, to_id));
    }

    let ax = map.graph[from_idx].center_coordinates.x;
    let ay = map.graph[from_idx].center_coordinates.y;
    let bx = map.graph[to_idx].center_coordinates.x;
    let by = map.graph[to_idx].center_coordinates.y;
    let dx = bx - ax;
    let dy = by - ay;
    let from_radius = map.graph[from_idx].radius;
    let to_radius = map.graph[to_idx].radius;
    let length = ((dx * dx + dy * dy).sqrt() - from_radius - to_radius).max(1.0);

    let road_id = map.add_road(from_id, to_id, lane_count, speed_limit, length);

    Ok(road_id)
}

pub fn delete_road(map: &mut Map, id: u32) -> Result<(), String> {
    let edge_idx = map
        .find_edge(id)
        .ok_or_else(|| format!("Road {} not found", id))?;

    map.graph.remove_edge(edge_idx);

    Ok(())
}

pub fn update_road(
    map: &mut Map,
    id: u32,
    speed_limit: f32,
) -> Result<(), String> {
    let edge_idx = map
        .find_edge(id)
        .ok_or_else(|| format!("Road {} not found", id))?;

    let road = &mut map.graph[edge_idx];
    road.speed_limit = speed_limit.clamp(1.0, MAX_SPEED);

    Ok(())
}

pub fn add_roundabout(
    map: &mut Map,
    center_x: f32,
    center_y: f32,
    ring_radius: f32,
    num_arms: usize,
    ring_speed_limit: f32,
    ring_lane_count: u8,
) -> RoundaboutHandle {
    assert!(num_arms >= 3, "A roundabout needs at least 3 arms");
    assert!(ring_radius > 0.0, "ring_radius must be positive");
    assert!(ring_lane_count >= 1, "ring_lane_count must be at least 1");

    let min_radius = 20.0_f32 / (std::f32::consts::TAU / num_arms as f32).sin();
    assert!(
        ring_radius >= min_radius,
        "ring_radius {ring_radius:.1} is too small for {num_arms} arms (minimum {min_radius:.1})"
    );

    let mut ring_node_ids = Vec::with_capacity(num_arms);
    for i in 0..num_arms {
        let angle = std::f32::consts::TAU * i as f32 / num_arms as f32;
        let x = center_x + ring_radius * angle.sin();
        let y = center_y - ring_radius * angle.cos();
        let id = map.add_intersection(IntersectionKind::Intersection, x, y);
        ring_node_ids.push(id);
    }

    let chord = 2.0 * ring_radius * (std::f32::consts::PI / num_arms as f32).sin();

    let mut ring_road_ids = Vec::with_capacity(num_arms);
    for i in 0..num_arms {
        let from = ring_node_ids[(i + 1) % num_arms];
        let to = ring_node_ids[i];
        let id = map.add_road(from, to, ring_lane_count, ring_speed_limit, chord);
        ring_road_ids.push(id);
    }

    RoundaboutHandle { ring_node_ids, ring_road_ids }
}

pub fn add_traffic_light_controller(
    map: &mut Map,
    intersection_id: u32,
    phases: Vec<(Vec<u32>, f32, f32)>,
) -> Result<TrafficLightControllerHandle, String> {
    if !map.node_index_map.contains_key(&intersection_id) {
        return Err(format!("Intersection {} not found", intersection_id));
    }
    if phases.is_empty() {
        return Err("Traffic light controller must have at least one phase".to_string());
    }

    let all_link_ids: Vec<u32> = phases.iter().flat_map(|(ids, _, _)| ids.iter().copied()).collect();

    for edge_idx in map.graph.edge_indices() {
        for lane in &mut map.graph[edge_idx].lanes {
            for link in &mut lane.links {
                if all_link_ids.contains(&link.id) {
                    link.link_type = LinkType::TrafficLight;
                }
                for foe in &mut link.foe_links {
                    if all_link_ids.contains(&foe.id) {
                        foe.link_type = LinkType::TrafficLight;
                    }
                }
            }
        }
    }

    let controller_id = map.next_controller_id;
    map.next_controller_id += 1;

    let signal_phases = phases
        .into_iter()
        .map(|(green_link_ids, green_duration, yellow_duration)| SignalPhase {
            green_link_ids,
            green_duration,
            yellow_duration,
        })
        .collect();

    let controller = TrafficLightController {
        id: controller_id,
        intersection_id,
        phases: signal_phases,
    };
    map.traffic_lights.insert(controller_id, controller);

    Ok(TrafficLightControllerHandle { controller_id })
}