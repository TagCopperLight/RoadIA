use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;

use crate::map::intersection::{IntersectionKind, IntersectionRules, IntersectionType};
use crate::map::model::Map;
use crate::simulation::config::MAX_SPEED;

pub fn add_node(map: &mut Map, x: f32, y: f32, kind: IntersectionKind, name: String) -> u32 {
    let idx = map.add_intersection(kind, name, x, y, IntersectionType::Priority);
    map.graph[idx].id
}

pub fn delete_node(map: &mut Map, id: u32) -> Result<(), String> {
    let idx = map
        .node_index_map
        .get(&id)
        .copied()
        .ok_or_else(|| format!("Node {} not found", id))?;

    let outgoing: Vec<(u32, u32)> = map
        .graph
        .edges_directed(idx, Direction::Outgoing)
        .map(|e| (e.weight().id, map.graph[e.target()].id))
        .collect();

    map.node_index_map.remove(&id);
    map.graph.remove_node(idx);

    if let Some(swapped) = map.graph.node_weight(idx) {
        let swapped_id = swapped.id;
        map.node_index_map.insert(swapped_id, idx);
    }

    for (edge_id, dest_id) in outgoing {
        if let Some(&dest_idx) = map.node_index_map.get(&dest_id) {
            map.graph[dest_idx].rules.remove(&edge_id);
            recalculate_intersection_rules(map, dest_idx);
        }
    }

    Ok(())
}

pub fn move_node(map: &mut Map, id: u32, x: f32, y: f32) -> Result<(), String> {
    let idx = map
        .node_index_map
        .get(&id)
        .copied()
        .ok_or_else(|| format!("Node {} not found", id))?;

    map.graph[idx].x = x;
    map.graph[idx].y = y;

    // Recalculate lengths of all connected edges.
    let edge_indices: Vec<_> = map
        .graph
        .edges_directed(idx, Direction::Incoming)
        .chain(map.graph.edges_directed(idx, Direction::Outgoing))
        .map(|e| e.id())
        .collect();

    for edge_idx in edge_indices {
        let (a, b) = map.graph.edge_endpoints(edge_idx).unwrap();
        let ax = map.graph[a].x;
        let ay = map.graph[a].y;
        let bx = map.graph[b].x;
        let by = map.graph[b].y;
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
    name: String,
) -> Result<(), String> {
    let idx = map
        .node_index_map
        .get(&id)
        .copied()
        .ok_or_else(|| format!("Node {} not found", id))?;

    map.graph[idx].kind = kind;
    map.graph[idx].name = name;

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

    let ax = map.graph[from_idx].x;
    let ay = map.graph[from_idx].y;
    let bx = map.graph[to_idx].x;
    let by = map.graph[to_idx].y;
    let dx = bx - ax;
    let dy = by - ay;
    let length = (dx * dx + dy * dy).sqrt();

    let road_id = map.add_road(from_idx, to_idx, lane_count, speed_limit, length, false, false);

    Ok(road_id)
}

pub fn delete_road(map: &mut Map, id: u32) -> Result<(), String> {
    let edge_idx = map
        .find_edge_by_id(id)
        .ok_or_else(|| format!("Road {} not found", id))?;

    let (from_idx, to_idx) = map
        .graph
        .edge_endpoints(edge_idx)
        .ok_or_else(|| format!("Road {} has invalid endpoints", id))?;

    // Remove the road's rule from the destination intersection.
    map.graph[to_idx].rules.remove(&id);

    map.graph.remove_edge(edge_idx);

    // Recalculate intersection rules on both endpoints.
    recalculate_intersection_rules(map, from_idx);
    recalculate_intersection_rules(map, to_idx);

    Ok(())
}

pub fn update_road(
    map: &mut Map,
    id: u32,
    lane_count: u8,
    speed_limit: f32,
    is_blocked: bool,
    can_overtake: bool,
) -> Result<(), String> {
    let edge_idx = map
        .find_edge_by_id(id)
        .ok_or_else(|| format!("Road {} not found", id))?;

    let road = &mut map.graph[edge_idx];
    road.lane_count = lane_count;
    road.speed_limit = speed_limit.clamp(1.0, MAX_SPEED);
    road.is_blocked = is_blocked;
    road.can_overtake = can_overtake;

    Ok(())
}

fn recalculate_intersection_rules(map: &mut Map, node_idx: NodeIndex) {
    let incoming_count = map
        .graph
        .edges_directed(node_idx, Direction::Incoming)
        .count();

    let edge_ids: Vec<u32> = map
        .graph
        .edges_directed(node_idx, Direction::Incoming)
        .map(|e| e.weight().id)
        .collect();

    if incoming_count <= 1 {
        map.graph[node_idx].intersection_type = IntersectionType::Priority;
        map.graph[node_idx].rules.clear();
        for road_id in edge_ids {
            map.graph[node_idx].rules.insert(road_id, IntersectionRules::Priority);
        }
    } else {
        map.graph[node_idx].rules.retain(|road_id, _| edge_ids.contains(road_id));

        let default_rule = match map.graph[node_idx].intersection_type {
            IntersectionType::Priority => IntersectionRules::Priority,
            IntersectionType::Stop => IntersectionRules::Stop,
            IntersectionType::TrafficLight => IntersectionRules::TrafficLight,
        };
        for road_id in edge_ids {
            map.graph[node_idx]
                .rules
                .entry(road_id)
                .or_insert(default_rule.clone());
        }
    }
}
