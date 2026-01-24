use petgraph::graph::{EdgeIndex, Graph, NodeIndex};
use serde_json::json;

use crate::map::intersection::Intersection;
use crate::map::road::Road;

pub struct Map {
    pub graph: Graph<Intersection, Road>,
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
            .map(|edge| self.graph[edge].length_m)
    }

    pub fn index_from_id(&self, id: u32) -> NodeIndex {
        self.graph
            .node_indices()
            .find(|i| self.graph[*i].id == id)
            .unwrap()
    }

    pub fn to_json(&self) -> serde_json::Value {
        let nodes: Vec<_> = self
            .graph
            .node_indices()
            .map(|i| {
                let n = &self.graph[i];
                json!({
                    "id": n.id,
                    "name": n.name,
                    "kind": n.kind,
                    "x": n.x,
                    "y": n.y
                })
            })
            .collect();

        let edges: Vec<_> = self
            .graph
            .edge_indices()
            .map(|e| {
                let (a, b) = self
                    .graph
                    .edge_endpoints(e)
                    .expect("edge_endpoints returned None for an EdgeIndex produced by edge_indices()");
                let r = &self.graph[e];
                json!({
                    "from": self.graph[a].id,
                    "to": self.graph[b].id,
                    "id": r.id,
                    "length": r.length_m,
                })
            })
            .collect();

        json!({
            "nodes": nodes,
            "edges": edges
        })
    }
}
