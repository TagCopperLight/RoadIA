use std::collections::{HashMap, HashSet, VecDeque};
use petgraph::graph::{EdgeIndex, Graph, NodeIndex};

use crate::map::intersection::{Intersection, IntersectionKind};
use crate::map::road::Road;
use crate::map::traffic_light::TrafficLightController;

#[derive(Default, Clone)]
pub struct Map {
    pub graph: Graph<Intersection, Road>,
    pub node_index_map: HashMap<u32, NodeIndex>,
    pub next_node_id: u32,
    pub next_edge_id: u32,
    pub next_link_id: u32,
    pub next_controller_id: u32,
    pub traffic_lights: HashMap<u32, TrafficLightController>,
}

#[derive(Clone)]
pub struct Coordinates{
    pub x : f32,
    pub y : f32,
}

impl Map {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
            node_index_map: HashMap::new(),
            next_node_id: 0,
            next_edge_id: 0,
            next_link_id: 0,
            next_controller_id: 0,
            traffic_lights: HashMap::new(),
        }
    }

    pub fn add_intersection(
        &mut self,
        kind: IntersectionKind,
        x: f32,
        y: f32,
    ) -> u32 {
        let id = self.next_node_id;
        self.next_node_id += 1;
        
        let intersection = Intersection::new(id, kind, Coordinates { x, y }, 10.0);
        let idx = self.graph.add_node(intersection);
        self.node_index_map.insert(id, idx);
        id
    }

    pub fn add_road(
        &mut self,
        from: u32,
        to: u32,
        lane_count: u8,
        speed_limit: f32,
        length: f32,
    ) -> u32 {
        let id = self.next_edge_id;
        self.next_edge_id += 1;
        
        let from_node = self.find_node(from).expect("Start intersection not found");
        let to_node = self.find_node(to).expect("End intersection not found");
        
        let road = Road::new(id, lane_count, speed_limit, length);
        self.graph.add_edge(from_node, to_node, road);
        id
    }

    pub fn add_two_way_road(
        &mut self,
        from: u32,
        to: u32,
        lane_count: u8,
        speed_limit: f32,
        length: f32,
    ) -> (u32, u32) {
        let id1 = self.add_road(from, to, lane_count, speed_limit, length);
        let id2 = self.add_road(to, from, lane_count, speed_limit, length);
        (id1, id2)
    }

    pub fn find_node(&self, id: u32) -> Option<NodeIndex> {
        self.node_index_map.get(&id).copied()
    }

    pub fn find_edge(&self, id: u32) -> Option<EdgeIndex> {
        self.graph.edge_indices().find(|&e| self.graph[e].id == id)
    }

    pub fn neighboring_intersections(&self, source: NodeIndex) -> Vec<NodeIndex> {
        self.graph.neighbors(source).collect()
    }

    pub fn intersection_neighbor_distance(
        &self,
        source: NodeIndex,
        destination: NodeIndex,
    ) -> Option<f32> {
        self.graph
            .find_edge(source, destination)
            .map(|edge| self.graph[edge].length)
    }

    pub fn intersections_euclidean_distance(
        &self,
        source: NodeIndex,
        destination: NodeIndex,
    ) -> f32 {
        let n1 = &self.graph[source];
        let n2 = &self.graph[destination];
        let dx = n1.center_coordinates.x - n2.center_coordinates.x;
        let dy = n1.center_coordinates.y - n2.center_coordinates.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Keep only the largest weakly connected component of the graph.
    ///
    /// OSM data often contains small disconnected road fragments.
    /// This method removes them so that every node is reachable from
    /// every other node (treating edges as undirected for connectivity).
    pub fn retain_largest_component(&mut self) {
        let all_nodes: Vec<NodeIndex> = self.graph.node_indices().collect();
        if all_nodes.is_empty() {
            return;
        }

        // ── Find weakly connected components via BFS ────────────────
        // Build an undirected adjacency list from the directed graph.
        let mut adj: HashMap<NodeIndex, Vec<NodeIndex>> = HashMap::new();
        for edge in self.graph.edge_indices() {
            if let Some((a, b)) = self.graph.edge_endpoints(edge) {
                adj.entry(a).or_default().push(b);
                adj.entry(b).or_default().push(a);
            }
        }

        let mut visited: HashSet<NodeIndex> = HashSet::new();
        let mut components: Vec<HashSet<NodeIndex>> = Vec::new();

        for &start in &all_nodes {
            if visited.contains(&start) {
                continue;
            }
            let mut component = HashSet::new();
            let mut queue = VecDeque::new();
            queue.push_back(start);
            visited.insert(start);

            while let Some(node) = queue.pop_front() {
                component.insert(node);
                if let Some(neighbors) = adj.get(&node) {
                    for &neighbor in neighbors {
                        if visited.insert(neighbor) {
                            queue.push_back(neighbor);
                        }
                    }
                }
            }
            components.push(component);
        }

        // ── Find the largest component ──────────────────────────────
        let largest = components
            .into_iter()
            .max_by_key(|c| c.len())
            .unwrap_or_default();

        let total = all_nodes.len();
        let kept = largest.len();
        if kept == total {
            return; // graph is already fully connected
        }

        println!(
            "Retaining largest connected component: {} / {} nodes ({} removed)",
            kept,
            total,
            total - kept
        );

        // ── Rebuild the graph with only the largest component ───────
        let mut new_map = Map::new();
        let mut old_to_new: HashMap<NodeIndex, u32> = HashMap::new();

        // Re-add nodes
        for &old_idx in &largest {
            let node = &self.graph[old_idx];
            let new_id = new_map.add_intersection(
                node.kind.clone(),
                node.center_coordinates.x,
                node.center_coordinates.y,
            );
            old_to_new.insert(old_idx, new_id);
        }

        // Re-add edges
        for edge in self.graph.edge_indices() {
            if let Some((a, b)) = self.graph.edge_endpoints(edge) {
                if let (Some(&new_a), Some(&new_b)) = (old_to_new.get(&a), old_to_new.get(&b)) {
                    let road = &self.graph[edge];
                    new_map.add_road(
                        new_a,
                        new_b,
                        road.lanes.len() as u8,
                        road.speed_limit,
                        road.length,
                    );
                }
            }
        }

        *self = new_map;
    }

}
