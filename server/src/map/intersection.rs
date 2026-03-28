use std::collections::HashMap;

use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;

use crate::map::model::{Coordinates, Map};
use crate::map::road::{FoeLink, Link, LinkType};
use crate::simulation::vehicle::{LaneId, Vehicle};

#[derive(Debug, Clone)]
pub enum IntersectionKind {
    Habitation,
    Intersection,
    Workplace,
}

#[derive(Clone, Debug)]
pub struct InternalLane {
    pub id: u32,
    pub from_lane_id: u32,
    pub to_lane_id: u32,
    pub length: f32,
    pub speed_limit: f32,
    pub entry: (f32, f32),
    pub exit: (f32, f32),
}

#[derive(Clone)]
pub struct Intersection {
    pub id: u32,
    pub kind: IntersectionKind,
    pub center_coordinates: Coordinates,
    pub radius: f32,
    pub internal_lanes: Vec<InternalLane>,
}

impl Intersection {
    pub fn new(
        id: u32,
        kind: IntersectionKind,
        center_coordinates: Coordinates,
        radius: f32,
    ) -> Self {
        Self {
            id,
            kind,
            center_coordinates,
            radius,
            internal_lanes: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ApproachData {
    pub arrival_time: f32,
    pub leave_time: f32,
    pub arrival_speed: f32,
    pub leave_speed: f32,
    pub will_pass: bool,
}

#[derive(Default)]
pub struct LinkState {
    pub approaching: HashMap<u64, ApproachData>, // vehicle_id -> data
}

pub fn build_intersections(map: &mut Map) {
    let junction_nodes: Vec<NodeIndex> = map.graph.node_indices().collect();
    for node in junction_nodes {
        build_intersection(map, node);
    }
}

fn build_intersection(map: &mut Map, junction: NodeIndex) {
    let incoming: Vec<petgraph::graph::EdgeIndex> = map
        .graph
        .edges_directed(junction, petgraph::Direction::Incoming)
        .map(|e| e.id())
        .collect();

    let outgoing: Vec<petgraph::graph::EdgeIndex> = map
        .graph
        .edges_directed(junction, petgraph::Direction::Outgoing)
        .map(|e| e.id())
        .collect();

    if incoming.is_empty() || outgoing.is_empty() {
        return;
    }

    let (jx, jy, jradius) = {
        let j = &map.graph[junction];
        (j.center_coordinates.x, j.center_coordinates.y, j.radius)
    };

    struct RawLink {
        in_edge: petgraph::graph::EdgeIndex,
        out_edge: petgraph::graph::EdgeIndex,
        in_lane_idx: usize,
        from_lane_id: u32,
        to_lane_id: u32,
        destination_road_id: u32,
        internal_lane_id: u32,
        link_id: u32,
        entry: (f32, f32),
        exit: (f32, f32),
        length: f32,
        speed_limit: f32,
    }

    let mut raw: Vec<RawLink> = Vec::new();

    for &in_edge in &incoming {
        for &out_edge in &outgoing {
            let (in_src, _) = map.graph.edge_endpoints(in_edge).unwrap();
            let (_, out_dst) = map.graph.edge_endpoints(out_edge).unwrap();
            if in_src == out_dst {
                continue; // U-turn
            }

            let (in_lane_count, in_speed, in_lane_width) = {
                let r = &map.graph[in_edge];
                (r.lanes.len(), r.speed_limit, r.lane_width)
            };
            let (out_lane_count, out_speed, destination_road_id, out_lane_width) = {
                let r = &map.graph[out_edge];
                (r.lanes.len(), r.speed_limit, r.id, r.lane_width)
            };

            let (sx, sy) = node_coords(map, in_src);
            let (dx, dy) = node_coords(map, out_dst);

            let base_entry = boundary_point(jx, jy, jradius, sx, sy);
            let base_exit  = boundary_point(jx, jy, jradius, dx, dy);

            let in_perp  = perp_right(jx - sx, jy - sy);
            let out_perp = perp_right(dx - jx, dy - jy);

            for in_lane_idx in 0..in_lane_count {
                let from_lane_id = map.graph[in_edge].lanes[in_lane_idx].id;
                let entry = lane_boundary_point(base_entry, in_perp, in_lane_idx, in_lane_width);

                for out_lane_idx in 0..out_lane_count {
                    let to_lane_id = map.graph[out_edge].lanes[out_lane_idx].id;
                    let exit = lane_boundary_point(base_exit, out_perp, out_lane_idx, out_lane_width);

                    let length = dist(entry, exit).max(0.1);
                    let speed_limit = in_speed.min(out_speed);
                    let link_id = map.next_link_id;
                    map.next_link_id += 1;

                    raw.push(RawLink {
                        in_edge,
                        out_edge,
                        in_lane_idx,
                        from_lane_id,
                        to_lane_id,
                        destination_road_id,
                        internal_lane_id: 0, // placeholder
                        link_id,
                        entry,
                        exit,
                        length,
                        speed_limit,
                    });
                }
            }
        }
    }

    if raw.is_empty() {
        return;
    }

    let base = map.graph[junction].internal_lanes.len() as u32;
    for (i, r) in raw.iter_mut().enumerate() {
        r.internal_lane_id = base + i as u32;
    }
    let n = raw.len();
    let mut foes: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        for j in (i + 1)..n {
            let a = &raw[i];
            let b = &raw[j];
            let same_incoming = a.in_edge == b.in_edge && a.in_lane_idx == b.in_lane_idx;
            let crossing = segments_intersect(a.entry, a.exit, b.entry, b.exit);
            let merge = a.to_lane_id == b.to_lane_id && a.out_edge == b.out_edge;
            if !same_incoming && (crossing || merge) {
                foes[i].push(j);
                foes[j].push(i);
            }
        }
    }

    for r in &raw {
        map.graph[junction].internal_lanes.push(InternalLane {
            id: r.internal_lane_id,
            from_lane_id: r.from_lane_id,
            to_lane_id: r.to_lane_id,
            length: r.length,
            speed_limit: r.speed_limit,
            entry: r.entry,
            exit: r.exit,
        });
    }

    for (i, r) in raw.iter().enumerate() {
        let foe_links: Vec<FoeLink> = foes[i]
            .iter()
            .map(|&fi| FoeLink {
                id: raw[fi].link_id,
                link_type: LinkType::Priority,
                entry: raw[fi].entry,
            })
            .collect();
        let foe_internal_lane_ids: Vec<u32> =
            foes[i].iter().map(|&fi| raw[fi].internal_lane_id).collect();

        let link = Link {
            id: r.link_id,
            lane_origin_id: r.from_lane_id,
            lane_destination_id: r.to_lane_id,
            via_internal_lane_id: r.internal_lane_id,
            destination_road_id: r.destination_road_id,
            link_type: LinkType::Priority,
            entry: r.entry,
            junction_center: (jx, jy),
            foe_links,
            foe_internal_lane_ids,
        };

        map.graph[r.in_edge].lanes[r.in_lane_idx].links.push(link);
    }
}

pub(crate) fn node_coords(map: &Map, n: NodeIndex) -> (f32, f32) {
    let node = &map.graph[n];
    (node.center_coordinates.x, node.center_coordinates.y)
}

pub(crate) fn boundary_point(jx: f32, jy: f32, radius: f32, px: f32, py: f32) -> (f32, f32) {
    let dx = px - jx;
    let dy = py - jy;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return (jx, jy);
    }
    (jx + dx / len * radius, jy + dy / len * radius)
}

pub(crate) fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    (dx * dx + dy * dy).sqrt()
}

pub fn segments_intersect(
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    p4: (f32, f32),
) -> bool {
    let d1 = cross(p3, p4, p1);
    let d2 = cross(p3, p4, p2);
    let d3 = cross(p1, p2, p3);
    let d4 = cross(p1, p2, p4);

    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0))
        && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0))
    {
        return true;
    }

    if d1 == 0.0 && on_segment(p3, p4, p1) {
        return true;
    }
    if d2 == 0.0 && on_segment(p3, p4, p2) {
        return true;
    }
    if d3 == 0.0 && on_segment(p1, p2, p3) {
        return true;
    }
    if d4 == 0.0 && on_segment(p1, p2, p4) {
        return true;
    }

    false
}

pub(crate) fn cross(o: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

pub(crate) fn on_segment(p: (f32, f32), q: (f32, f32), r: (f32, f32)) -> bool {
    r.0 <= p.0.max(q.0)
        && r.0 >= p.0.min(q.0)
        && r.1 <= p.1.max(q.1)
        && r.1 >= p.1.min(q.1)
}

pub fn is_link_open(
    link: &Link,
    vehicle: &Vehicle,
    ego_data: &ApproachData,
    link_states: &HashMap<u32, LinkState>,
    vehicles_by_lane: &HashMap<LaneId, Vec<usize>>,
    vehicles: &[Vehicle],
    junction_id: u32,
    look_ahead: f32,
    stop_dwell_time: f32,
) -> bool {
    if matches!(vehicle.current_lane, Some(LaneId::Internal(_, _))) {
        return true;
    }

    if link.link_type == LinkType::Stop && vehicle.waiting_time < stop_dwell_time {
        return false;
    }

    for foe_link in &link.foe_links {
        let must_yield = match (&link.link_type, &foe_link.link_type) {
            (LinkType::Priority, LinkType::Yield) | (LinkType::Priority, LinkType::Stop) => false,
            (LinkType::Yield, LinkType::Priority) | (LinkType::Stop, LinkType::Priority) => true,
            _ => foe_is_to_the_right(link, foe_link),
        };

        if !must_yield {
            continue;
        }

        if let Some(foe_state) = link_states.get(&foe_link.id) {
            for (&foe_id, foe_data) in &foe_state.approaching {
                if foe_id == vehicle.id {
                    continue;
                }
                let foe_decel = vehicles
                    .iter()
                    .find(|v| v.id == foe_id)
                    .map(|v| v.spec.comfortable_deceleration)
                    .unwrap_or(3.0);

                if time_window_conflict(
                    ego_data.arrival_time,
                    ego_data.leave_time,
                    foe_data.arrival_time,
                    foe_data.leave_time,
                    foe_data.leave_speed,
                    foe_data.arrival_speed,
                    foe_decel,
                    look_ahead,
                    false,
                    vehicle.impatience,
                ) {
                    return false;
                }
            }
        }
    }

    for &foe_int_lane_id in &link.foe_internal_lane_ids {
        let key = LaneId::Internal(junction_id, foe_int_lane_id);
        if vehicles_by_lane.get(&key).is_some_and(|v| !v.is_empty()) {
            return false;
        }
    }

    true
}

pub(crate) fn foe_is_to_the_right(ego: &Link, foe: &FoeLink) -> bool {
    let ex = ego.junction_center.0 - ego.entry.0;
    let ey = ego.junction_center.1 - ego.entry.1;
    let fx = ego.junction_center.0 - foe.entry.0;
    let fy = ego.junction_center.1 - foe.entry.1;
    ex * fy - ey * fx < 0.0
}

pub(crate) fn time_window_conflict(
    ego_arrival: f32,
    ego_leave: f32,
    foe_arrival: f32,
    foe_leave: f32,
    foe_leave_speed: f32,
    foe_approach_speed: f32,
    _ego_max_decel: f32,
    look_ahead: f32,
    same_target: bool,
    ego_impatience: f32,
) -> bool {
    let foe_arrival_adj = lerp(foe_arrival, foe_arrival + look_ahead * 2.0, ego_impatience);
    let foe_leave_adj = foe_leave + (foe_arrival_adj - foe_arrival);

    if ego_arrival < foe_leave_adj && foe_arrival_adj < ego_leave {
        return true;
    }

    if foe_leave_adj < ego_arrival {
        let gap = ego_arrival - foe_leave_adj;
        if gap < look_ahead {
            return true;
        }
        if same_target && foe_leave_speed < 1.0 {
            return true;
        }
        return false;
    }

    if ego_leave + look_ahead < foe_arrival_adj {
        if same_target && foe_approach_speed > foe_leave_speed + 2.0 {
            return true;
        }
        return false;
    }

    true
}

pub(crate) fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}

pub(crate) fn perp_right(dx: f32, dy: f32) -> (f32, f32) {
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-6 {
        return (1.0, 0.0);
    }
    (-dy / len, dx / len)
}

pub(crate) fn lane_boundary_point(
    base: (f32, f32),
    perp: (f32, f32),
    lane_idx: usize,
    lane_width: f32,
) -> (f32, f32) {
    let offset = (lane_idx as f32 + 0.5) * lane_width;
    (base.0 + perp.0 * offset, base.1 + perp.1 * offset)
}