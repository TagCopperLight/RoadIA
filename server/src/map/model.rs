use petgraph::graph::{Graph, NodeIndex};
use serde_json::json;

use crate::map::intersection::{Intersection, IntersectionKind};
use crate::map::road::Road;

#[derive(Clone)]
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

    pub fn add_road(&mut self, from: NodeIndex, to: NodeIndex, road: Road) {
        self.graph.add_edge(from, to, road.clone());
        // Since we had Bilateral/Unilateral in RoadType but old code added edge both ways unconditionally for add_road.
        // Waiting, the old code:
        // pub fn add_road(&mut self, from: NodeIndex, to: NodeIndex, seg: RoadSegment) {
        //     self.graph.add_edge(from, to, seg.clone());
        //     self.graph.add_edge(to, from, seg);
        // }
        // It always added both ways.
        self.graph.add_edge(to, from, road);
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
                    "kind": format!("{:?}", n.kind),
                    "x": n.x,
                    "y": n.y
                })
            })
            .collect();

        let edges: Vec<_> = self
            .graph
            .edge_indices()
            .map(|e| {
                let (a, b) = self.graph.edge_endpoints(e).unwrap();
                let seg = self.graph.edge_weight(e).unwrap();
                json!({
                    "from": self.graph[a].id,
                    "to": self.graph[b].id,
                    "length": seg.length_m
                })
            })
            .collect();

        json!({
            "nodes": nodes,
            "edges": edges
        })
    }
}
