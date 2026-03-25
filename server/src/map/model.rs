use std::collections::HashMap;
use petgraph::graph::{EdgeIndex, Graph, NodeIndex};

use crate::map::intersection::{Intersection, IntersectionKind};
use crate::map::road::Road;

#[derive(Default, Clone)]
pub struct Map {
    pub graph: Graph<Intersection, Road>,
    pub node_index_map: HashMap<u32, NodeIndex>,
    pub next_node_id: u32,
    pub next_edge_id: u32,
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
        
        let intersection = Intersection::new(id, kind, Coordinates { x, y });
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
}
