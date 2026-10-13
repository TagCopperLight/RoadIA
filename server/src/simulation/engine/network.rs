//! Static view of the road network, derived from the map once per edit and indexed for
//! the hot paths of the simulation (links by id, internal lanes, link choice per lane).

use std::collections::HashMap;

use rustc_hash::FxHashMap;

use petgraph::graph::EdgeIndex;
use petgraph::visit::EdgeRef;

use crate::map::model::Map;
use crate::map::road::{Link, LinkType};
use crate::simulation::vehicle::LaneId;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Turn {
    Straight,
    Left,
    Right,
    /// U-turn.
    Back,
}

impl Turn {
    fn crosses_oncoming_traffic(self) -> bool {
        matches!(self, Turn::Left | Turn::Back)
    }
}

/// A link with everything the simulation needs to know about it.
#[derive(Clone)]
pub(crate) struct LinkInfo {
    pub link: Link,
    pub junction_id: u32,
    pub in_lane: LaneId,
    pub out_lane: LaneId,
    pub internal_lane: LaneId,
    /// Length of the internal lane crossing the junction.
    pub length: f32,
    pub speed_limit: f32,
    /// Unit vector of the direction of the incoming road.
    pub heading: (f32, f32),
    pub turn: Turn,
    /// Whether a vehicle stopped inside the junction on this link would block a crossing
    /// flow that does not yield to it. Such links are only entered when the exit has room.
    pub keep_clear: bool,
}

impl LinkInfo {
    /// Right-of-way class of the link: a vehicle yields to foes of a higher rank.
    pub fn rank(&self) -> u8 {
        match self.link.link_type {
            LinkType::Stop => 0,
            LinkType::Yield => 1,
            LinkType::Priority | LinkType::TrafficLight => 2,
        }
    }

    /// Whether a vehicle using `self` must give way to a vehicle using `foe`.
    /// For any pair of distinct links exactly one of them yields, which keeps the
    /// precedence relation free of ties.
    pub fn must_yield_to(&self, foe: &LinkInfo) -> bool {
        if self.rank() != foe.rank() {
            return self.rank() < foe.rank();
        }

        let along = dot(self.heading, foe.heading);
        let side = cross(self.heading, foe.heading);

        let yields = if along < -0.7 {
            // Oncoming traffic: turning across it means giving way.
            match (self.turn.crosses_oncoming_traffic(), foe.turn.crosses_oncoming_traffic()) {
                (true, false) => Some(true),
                (false, true) => Some(false),
                _ => None,
            }
        } else if along > 0.7 {
            // Same direction (adjacent lanes or merging roads): the left one gives way.
            // Measured against the mean heading so that the rule is symmetric.
            let mean = (self.heading.0 + foe.heading.0, self.heading.1 + foe.heading.1);
            let offset = (foe.link.entry.0 - self.link.entry.0, foe.link.entry.1 - self.link.entry.1);
            let lateral = cross(mean, offset);
            (lateral.abs() > 1e-3).then_some(lateral > 0.0)
        } else {
            // Priority to the right (screen coordinates: y points down).
            (side.abs() > 1e-6).then_some(side < 0.0)
        };

        yields.unwrap_or(self.link.id > foe.link.id)
    }
}

#[derive(Clone, Default)]
pub(crate) struct Network {
    links: Vec<Option<LinkInfo>>,
    link_by_internal_lane: FxHashMap<LaneId, u32>,
    /// Preferred link for each (incoming edge, incoming lane, outgoing edge).
    link_choice: FxHashMap<(EdgeIndex, u32, EdgeIndex), u32>,
}

impl Network {
    pub fn build(map: &Map) -> Self {
        let edge_by_road_id: HashMap<u32, EdgeIndex> = map
            .graph
            .edge_indices()
            .map(|edge| (map.graph[edge].id, edge))
            .collect();

        let mut network = Network::default();

        let position = |node: petgraph::graph::NodeIndex| {
            let coordinates = &map.graph[node].center_coordinates;
            (coordinates.x, coordinates.y)
        };
        let direction = |edge: EdgeIndex| {
            let (from, to) = map.graph.edge_endpoints(edge).unwrap();
            let (from, to) = (position(from), position(to));
            normalize((to.0 - from.0, to.1 - from.1))
        };

        for edge in map.graph.edge_references() {
            let in_edge = edge.id();
            let junction = &map.graph[edge.target()];
            let road = edge.weight();
            let heading = direction(in_edge);

            for lane in &road.lanes {
                for link in &lane.links {
                    let Some(&out_edge) = edge_by_road_id.get(&link.destination_road_id) else {
                        continue;
                    };
                    let Some(internal) = junction
                        .internal_lanes
                        .iter()
                        .find(|il| il.id == link.via_internal_lane_id)
                    else {
                        continue;
                    };

                    let info = LinkInfo {
                        link: link.clone(),
                        junction_id: junction.id,
                        in_lane: LaneId::Normal(in_edge, lane.id),
                        out_lane: LaneId::Normal(out_edge, link.lane_destination_id),
                        internal_lane: LaneId::Internal(junction.id, internal.id),
                        length: internal.length,
                        speed_limit: internal.speed_limit,
                        heading,
                        turn: classify_turn(heading, direction(out_edge)),
                        keep_clear: true,
                    };

                    let out_lane_count = map.graph[out_edge].lanes.len() as u32;
                    let preferred = lane.id.min(out_lane_count.saturating_sub(1));
                    network
                        .link_choice
                        .entry((in_edge, lane.id, out_edge))
                        .and_modify(|chosen| {
                            if link.lane_destination_id == preferred {
                                *chosen = link.id;
                            }
                        })
                        .or_insert(link.id);

                    network.link_by_internal_lane.insert(info.internal_lane, link.id);
                    let slot = link.id as usize;
                    if network.links.len() <= slot {
                        network.links.resize(slot + 1, None);
                    }
                    network.links[slot] = Some(info);
                }
            }
        }

        network.compute_keep_clear();
        network
    }

    fn compute_keep_clear(&mut self) {
        let keep_clear: Vec<bool> = self
            .links
            .iter()
            .map(|info| {
                info.as_ref().is_some_and(|info| {
                    info.link.foe_links.iter().any(|foe| {
                        self.link(foe.id).is_some_and(|foe| {
                            foe.out_lane != info.out_lane && foe.rank() >= info.rank()
                        })
                    })
                })
            })
            .collect();
        for (info, keep_clear) in self.links.iter_mut().zip(keep_clear) {
            if let Some(info) = info {
                info.keep_clear = keep_clear;
            }
        }
    }

    /// Exclusive upper bound of link ids.
    pub fn link_id_bound(&self) -> usize {
        self.links.len()
    }

    pub fn link(&self, link_id: u32) -> Option<&LinkInfo> {
        self.links.get(link_id as usize).and_then(Option::as_ref)
    }

    pub fn link_of_internal_lane(&self, lane: LaneId) -> Option<&LinkInfo> {
        self.link_by_internal_lane
            .get(&lane)
            .and_then(|&id| self.link(id))
    }

    /// The link a vehicle on `in_lane` of `in_edge` takes to reach `out_edge`. Falls back to
    /// a link of another lane of the same road when the lane itself has none.
    pub fn choose_link(
        &self,
        map: &Map,
        in_edge: EdgeIndex,
        in_lane: u32,
        out_edge: EdgeIndex,
    ) -> Option<&LinkInfo> {
        let lane_count = map.graph.edge_weight(in_edge)?.lanes.len() as u32;
        std::iter::once(in_lane)
            .chain((0..lane_count).filter(|&lane| lane != in_lane))
            .find_map(|lane| self.link_choice.get(&(in_edge, lane, out_edge)))
            .and_then(|&id| self.link(id))
    }

    pub fn lane_length(&self, map: &Map, lane: LaneId) -> f32 {
        match lane {
            LaneId::Normal(edge, _) => map.graph.edge_weight(edge).map_or(0.0, |road| road.length),
            LaneId::Internal(..) => self.link_of_internal_lane(lane).map_or(0.0, |info| info.length),
        }
    }

    pub fn lane_speed_limit(&self, map: &Map, lane: LaneId) -> f32 {
        match lane {
            LaneId::Normal(edge, _) => map
                .graph
                .edge_weight(edge)
                .map_or(crate::simulation::config::MAX_SPEED, |road| road.speed_limit),
            LaneId::Internal(..) => self
                .link_of_internal_lane(lane)
                .map_or(crate::simulation::config::MAX_SPEED, |info| info.speed_limit),
        }
    }
}

fn classify_turn(heading: (f32, f32), exit_dir: (f32, f32)) -> Turn {
    let side = cross(heading, exit_dir);
    if dot(heading, exit_dir) < -0.7 {
        Turn::Back
    } else if side < -0.5 {
        Turn::Left
    } else if side > 0.5 {
        Turn::Right
    } else {
        Turn::Straight
    }
}

fn normalize(v: (f32, f32)) -> (f32, f32) {
    let len = (v.0 * v.0 + v.1 * v.1).sqrt();
    if len < 1e-6 {
        (1.0, 0.0)
    } else {
        (v.0 / len, v.1 / len)
    }
}

fn dot(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1
}

fn cross(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.1 - a.1 * b.0
}
