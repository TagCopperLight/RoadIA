use petgraph::graph::{EdgeIndex, Graph, NodeIndex};

use crate::map::intersection::Intersection;
use crate::map::road::Road;

#[derive(Default, Clone, Debug)]
pub struct Map {
    pub graph: Graph<Intersection, Road>,
}

#[derive(Debug, Clone)]
pub struct Coordinates {
    pub x: f32,
    pub y: f32,
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
}
