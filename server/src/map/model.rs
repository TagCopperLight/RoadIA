use petgraph::graph::{Graph, NodeIndex};
use serde_json::json;

use crate::map::intersection::{Intersection, IntersectionKind};
use crate::map::road::Road;

pub enum Node{
    Road(Road),
    Intersection(Intersection),
}

#[derive(Clone)]
pub struct Map {
    pub graph: Graph<Node, ()>,
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

    pub fn add_road(&mut self, from: NodeIndex, to: NodeIndex, road: Road) -> NodeIndex{
        let roadIndex = self.graph.add_node(road);
        self.graph.add_edge(from, roadIndex, {});
        self.graph.add_edge(roadIndex, from, {});
        self.graph.add_edge(roadIndex, to, {});
        self.graph.add_edge(to, roadIndex, {});
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
