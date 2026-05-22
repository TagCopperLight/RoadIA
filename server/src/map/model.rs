use std::collections::{HashMap, HashSet, VecDeque};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use petgraph::graph::{EdgeIndex, Graph, NodeIndex};

use crate::map::intersection::{Intersection, IntersectionKind};
use serde::{Serialize, Deserialize};
use crate::map::road::Road;
use crate::map::traffic_light::TrafficLightController;

#[derive(Clone, Serialize, Deserialize)]
pub struct MapSettings {
    #[serde(default = "MapSettings::default_vehicle_count")]
    pub vehicle_count: usize,
    #[serde(default = "MapSettings::default_simulation_duration")]
    pub simulation_duration: f32,
    #[serde(default = "MapSettings::default_simulation_start_time")]
    pub simulation_start_time: f32,
    #[serde(default = "MapSettings::default_time_step")]
    pub time_step: f32,
    #[serde(default = "MapSettings::default_max_budget")]
    pub max_budget: u64,
    #[serde(default = "MapSettings::default_base_cost_per_meter")]
    pub base_cost_per_meter: u32,
    #[serde(default = "MapSettings::default_intersection_cost")]
    pub intersection_cost: u32,
    #[serde(default = "MapSettings::default_habitation_cost")]
    pub habitation_cost: u32,
    #[serde(default = "MapSettings::default_workplace_cost")]
    pub workplace_cost: u32,
    #[serde(default = "MapSettings::default_time_weight")]
    pub time_weight: f32,
    #[serde(default = "MapSettings::default_success_weight")]
    pub success_weight: f32,
    #[serde(default = "MapSettings::default_pollution_weight")]
    pub pollution_weight: f32,
    #[serde(default = "MapSettings::default_infrastructure_weight")]
    pub infrastructure_weight: f32,
}

impl MapSettings {
    pub const DEFAULT_VEHICLE_COUNT: usize = 500;
    pub const DEFAULT_SIMULATION_DURATION_S: f32 = 86_400.0;
    pub const DEFAULT_SIMULATION_START_TIME_S: f32 = 0.0;
    pub const DEFAULT_TIME_STEP_S: f32 = 0.1;
    pub const MAX_SIMULATION_START_TIME_S: f32 = 39_600.0;
    pub const MAX_SIMULATION_DURATION_S: f32 = 86_400.0;

    fn default_vehicle_count() -> usize { Self::DEFAULT_VEHICLE_COUNT }
    fn default_simulation_duration() -> f32 { Self::DEFAULT_SIMULATION_DURATION_S }
    fn default_simulation_start_time() -> f32 { Self::DEFAULT_SIMULATION_START_TIME_S }
    fn default_time_step() -> f32 { Self::DEFAULT_TIME_STEP_S }
    fn default_max_budget() -> u64 { 750_000_000 }
    fn default_base_cost_per_meter() -> u32 { 500 }
    fn default_intersection_cost() -> u32 { 50_000 }
    fn default_habitation_cost() -> u32 { 150_000 }
    fn default_workplace_cost() -> u32 { 200_000 }
    fn default_time_weight() -> f32 { 0.4 }
    fn default_success_weight() -> f32 { 0.2 }
    fn default_pollution_weight() -> f32 { 0.2 }
    fn default_infrastructure_weight() -> f32 { 0.2 }
}

impl Default for MapSettings {
    fn default() -> Self {
        Self {
            vehicle_count: Self::default_vehicle_count(),
            simulation_duration: Self::default_simulation_duration(),
            simulation_start_time: Self::default_simulation_start_time(),
            time_step: Self::default_time_step(),
            max_budget: Self::default_max_budget(),
            base_cost_per_meter: Self::default_base_cost_per_meter(),
            intersection_cost: Self::default_intersection_cost(),
            habitation_cost: Self::default_habitation_cost(),
            workplace_cost: Self::default_workplace_cost(),
            time_weight: Self::default_time_weight(),
            success_weight: Self::default_success_weight(),
            pollution_weight: Self::default_pollution_weight(),
            infrastructure_weight: Self::default_infrastructure_weight(),
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct SavedBusLine {
    pub id: u64,
    pub name: String,
    pub stop_node_ids: Vec<u32>,
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct Map {
    pub graph: Graph<Intersection, Road>,
    pub node_index_map: HashMap<u32, NodeIndex>,
    pub next_node_id: u32,
    pub next_edge_id: u32,
    pub next_link_id: u32,
    pub next_controller_id: u32,
    pub traffic_lights: HashMap<u32, TrafficLightController>,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub settings: MapSettings,
    #[serde(default)]
    pub bus_lines: Vec<SavedBusLine>,
    #[serde(default)]
    pub next_bus_line_id: u64,
}

#[derive(Clone, Serialize, Deserialize)]
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
            name: String::new(),
            settings: MapSettings::default(),
            bus_lines: Vec::new(),
            next_bus_line_id: 0,
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
        
        let intersection = Intersection::new(id, kind, Coordinates { x, y }, 5.0);
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
        self.graph.retain_nodes(|_, n| largest.contains(&n));
        
        // Rebuild node_index_map since NodeIndex values shift during retain_nodes
        self.node_index_map.clear();
        for node_idx in self.graph.node_indices() {
            let id = self.graph[node_idx].id;
            self.node_index_map.insert(id, node_idx);
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut json = String::new();
        file.read_to_string(&mut json)?;
        let map: Self = serde_json::from_str(&json)?;
        Ok(map)
    }

}
