use std::collections::HashSet;

use petgraph::visit::EdgeRef;
use petgraph::Direction;

use crate::map::model::Map;
use crate::map::road::LinkType;

pub struct RoundaboutHandle {
    pub ring_node_ids: Vec<u32>,
    pub ring_road_ids: Vec<u32>,
}

pub fn finalize_roundabout_links(map: &mut Map, handle: &RoundaboutHandle) {
    let ring_node_set: HashSet<u32> = handle.ring_node_ids.iter().copied().collect();

    let mut entry_link_ids: HashSet<u32> = HashSet::new();
    for &ring_node_id in &handle.ring_node_ids {
        let ring_node_idx = map.find_node(ring_node_id).expect("ring node not found");
        let incoming: Vec<_> = map
            .graph
            .edges_directed(ring_node_idx, Direction::Incoming)
            .map(|e| e.id())
            .collect();

        for edge_idx in incoming {
            let (src, _) = map.graph.edge_endpoints(edge_idx).unwrap();
            let src_id = map.graph[src].id;
            if !ring_node_set.contains(&src_id) {
                for lane in &mut map.graph[edge_idx].lanes {
                    for link in &mut lane.links {
                        entry_link_ids.insert(link.id);
                        link.link_type = LinkType::Yield;
                    }
                }
            }
        }
    }

    for &ring_road_id in &handle.ring_road_ids {
        if let Some(edge_idx) = map.find_edge(ring_road_id) {
            for lane in &mut map.graph[edge_idx].lanes {
                for link in &mut lane.links {
                    for foe in &mut link.foe_links {
                        if entry_link_ids.contains(&foe.id) {
                            foe.link_type = LinkType::Yield;
                        }
                    }
                }
            }
        }
    }
}

