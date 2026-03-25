use petgraph::visit::EdgeRef;
use petgraph::Direction;

use crate::map::intersection::{IntersectionKind};
use crate::map::model::Map;
use crate::simulation::config::MAX_SPEED;

pub fn add_node(map: &mut Map, x: f32, y: f32, kind: IntersectionKind) -> u32 {
    let id = map.add_intersection(kind, x, y);
    id
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
        map.graph[edge_idx].length = (dx * dx + dy * dy).sqrt();
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
    let length = (dx * dx + dy * dy).sqrt();

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