use petgraph::graph::{EdgeIndex, Graph, NodeIndex};

use crate::map::intersection::Intersection;
use crate::map::road::Road;

#[derive(Default, Clone)]
pub struct Map {
    pub graph: Graph<Intersection, Road>,
}

pub struct Coordinates{
    pub x : f32,
    pub y : f32,
}

impl Map {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
        }
    }

    pub fn add_intersection(&mut self, intersection: Intersection) -> NodeIndex {
        self.graph.add_node(intersection)
    }

    pub fn add_road(&mut self, from: NodeIndex, to: NodeIndex, road: Road) -> EdgeIndex {
        self.graph.add_edge(from, to, road)
    }

    pub fn add_two_way_road(
        &mut self,
        from: NodeIndex,
        to: NodeIndex,
        road: Road,
    ) -> (EdgeIndex, EdgeIndex) {
        let e1 = self.add_road(from, to, road.clone());
        let e2 = self.add_road(to, from, road);
        (e1, e2)
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
        let dx = n1.x - n2.x;
        let dy = n1.y - n2.y;
        (dx * dx + dy * dy).sqrt()
    }
}
