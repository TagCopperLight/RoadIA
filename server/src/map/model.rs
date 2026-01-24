use petgraph::graph::{Graph, NodeIndex};
use serde_json::json;

use crate::map::intersection::{Intersection, IntersectionKind};
use crate::map::road::Road;

#[derive(Debug, Clone)]
pub enum Node{
    Road(Road),
    Intersection(Intersection),
}

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
        self.graph.add_node(Node::Intersection(intersection))
    }

    pub fn add_road(&mut self, from: NodeIndex, to: NodeIndex, road: Road) -> NodeIndex{
        let road_index = self.graph.add_node(Node::Road(road));
        self.graph.add_edge(from, road_index, {});
        self.graph.add_edge(road_index, from, {});
        self.graph.add_edge(road_index, to, {});
        self.graph.add_edge(to, road_index, {});
        return road_index;
    }

    pub fn neighboring_intersections(&self, source: NodeIndex) -> Vec<NodeIndex>{
        let mut neighbors_intersections = Vec::new();
        for road_index in self.graph.neighbors(source){
            for intersection_index in self.graph.neighbors(road_index){
                if intersection_index != source{
                    neighbors_intersections.push(intersection_index);
                }
            }
        }
        return neighbors_intersections;
    }

    pub fn intersection_neighbor_distance(&self, source : NodeIndex, destination : NodeIndex) -> f32{
        for road_index in self.graph.neighbors(source){
            let road_neighbors : Vec<NodeIndex> = self.graph.neighbors(road_index).collect();
            if road_neighbors.contains(&source) && road_neighbors.contains(&destination){
                if let Node::Road(r) = &self.graph[road_index]{
                    return r.length_m;
                }
            }
        }
        return -67.0; //pour que le putin de compiler ne casse pas les couilles
    }

    pub fn index_from_id(&self, id: u32) -> NodeIndex {
        self.graph
            .node_indices()
            .find(|i| match &self.graph[*i] {
                Node::Road(n) => n.id==id,
                Node::Intersection(n) => n.id==id,
            })
            .unwrap()
    }

    pub fn to_json(&self) -> serde_json::Value {
        let nodes: Vec<_> = self
            .graph
            .node_indices()
            .map(|i| {
                match &self.graph[i] {
                    Node::Road(n) =>
                        json!({
                            "id": n.id,
                        }),
                    Node::Intersection(n) => 
                        json!({
                            "id": n.id,
                            "name": n.name,
                            "x": n.x,
                            "y": n.y
                        }),
                };
            })
            .collect();

        let edges: Vec<_> = self
            .graph
            .edge_indices()
            .map(|e| {
                let (a, b) = self.graph.edge_endpoints(e).unwrap();
                json!({
                    "from": match &self.graph[a] {
                        Node::Road(n) => n.id,
                        Node::Intersection(n) => n.id,
                    },
                    "to": match &self.graph[b] {
                        Node::Road(n) => n.id,
                        Node::Intersection(n) => n.id,
                    }
                })
            })
            .collect();

        json!({
            "nodes": nodes,
            "edges": edges
        })
    }
}
