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
    let ring_road_ids: HashSet<u32> = handle.ring_road_ids.iter().copied().collect();
    mark_roundabout_entries(map, &ring_road_ids);
}

/// Makes traffic entering a roundabout yield to the traffic already on it: every link of a
/// road that ends on the ring without being part of it becomes a `Yield` link.
/// Must run after `build_intersections`, which resets link types. Unknown ids are ignored.
pub fn mark_roundabout_entries(map: &mut Map, ring_road_ids: &HashSet<u32>) {
    let ring_nodes: HashSet<_> = map
        .graph
        .edge_references()
        .filter(|edge| ring_road_ids.contains(&edge.weight().id))
        .map(|edge| edge.target())
        .collect();

    let entries: Vec<_> = ring_nodes
        .iter()
        .flat_map(|&node| map.graph.edges_directed(node, Direction::Incoming))
        .filter(|edge| !ring_road_ids.contains(&edge.weight().id))
        .map(|edge| edge.id())
        .collect();

    let mut entry_link_ids: HashSet<u32> = HashSet::new();
    for edge in entries {
        for lane in &mut map.graph[edge].lanes {
            for link in &mut lane.links {
                link.link_type = LinkType::Yield;
                entry_link_ids.insert(link.id);
            }
        }
    }

    for road in map.graph.edge_weights_mut() {
        for link in road.lanes.iter_mut().flat_map(|lane| &mut lane.links) {
            for foe in &mut link.foe_links {
                if entry_link_ids.contains(&foe.id) {
                    foe.link_type = LinkType::Yield;
                }
            }
        }
    }
}
