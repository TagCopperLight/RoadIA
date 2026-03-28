use std::collections::HashMap;

use petgraph::graph::NodeIndex;

use crate::map::intersection::{
    boundary_point, build_intersections, cross, dist, foe_is_to_the_right, is_link_open,
    lerp, on_segment, perp_right, lane_boundary_point, segments_intersect, time_window_conflict,
    ApproachData, IntersectionKind, LinkState,
};
use crate::map::model::Map;
use crate::map::road::{FoeLink, Link, LinkType};
use crate::simulation::vehicle::{LaneId, Vehicle};
use crate::test::{make_approach_data, make_default_link, make_foe_link, make_minimal_straight_map, make_vehicle};

// ---- cross ----

#[test]
fn cross_positive() {
    // (1,0) × (0,1) from origin → +1
    let v = cross((0.0, 0.0), (1.0, 0.0), (0.0, 1.0));
    assert!((v - 1.0).abs() < 1e-6, "got {v}");
}

#[test]
fn cross_negative() {
    let v = cross((0.0, 0.0), (0.0, 1.0), (1.0, 0.0));
    assert!((v - (-1.0)).abs() < 1e-6, "got {v}");
}

#[test]
fn cross_collinear() {
    let v = cross((0.0, 0.0), (1.0, 0.0), (2.0, 0.0));
    assert!(v.abs() < 1e-6, "expected 0, got {v}");
}

// ---- on_segment ----

#[test]
fn on_segment_midpoint() {
    assert!(on_segment((0.0, 0.0), (2.0, 0.0), (1.0, 0.0)));
}

#[test]
fn on_segment_at_endpoint() {
    assert!(on_segment((0.0, 0.0), (2.0, 0.0), (0.0, 0.0)));
}

#[test]
fn on_segment_outside() {
    assert!(!on_segment((0.0, 0.0), (2.0, 0.0), (3.0, 0.0)));
}

#[test]
fn on_segment_perpendicular_off() {
    // Point not on the x-axis segment
    assert!(!on_segment((0.0, 0.0), (2.0, 0.0), (1.0, 1.0)));
}

// ---- segments_intersect ----

#[test]
fn segments_intersect_crossing_x() {
    // Horizontal (0,1)-(2,1) crosses vertical (1,0)-(1,2)
    assert!(segments_intersect(
        (0.0, 1.0), (2.0, 1.0),
        (1.0, 0.0), (1.0, 2.0)
    ));
}

#[test]
fn segments_intersect_parallel_horizontal() {
    assert!(!segments_intersect(
        (0.0, 0.0), (2.0, 0.0),
        (0.0, 1.0), (2.0, 1.0)
    ));
}

#[test]
fn segments_intersect_t_junction() {
    // Endpoint of second segment touches first segment
    assert!(segments_intersect(
        (0.0, 0.0), (2.0, 0.0),
        (1.0, -1.0), (1.0, 0.0)
    ));
}

#[test]
fn segments_intersect_shared_endpoint() {
    assert!(segments_intersect(
        (0.0, 0.0), (1.0, 1.0),
        (1.0, 1.0), (2.0, 0.0)
    ));
}

#[test]
fn segments_intersect_no_intersection() {
    // Collinear but gap between them
    assert!(!segments_intersect(
        (0.0, 0.0), (1.0, 0.0),
        (2.0, 0.0), (3.0, 0.0)
    ));
}

#[test]
fn segments_intersect_diagonal_miss() {
    assert!(!segments_intersect(
        (0.0, 0.0), (1.0, 1.0),
        (2.0, 0.0), (3.0, 1.0)
    ));
}

// ---- boundary_point ----

#[test]
fn boundary_point_north() {
    // Junction at origin, neighbor directly north (+y in y-up / -y in y-down doesn't matter here)
    // px=0, py=100, radius=10 → point at (0, 10)
    let (bx, by) = boundary_point(0.0, 0.0, 10.0, 0.0, 100.0);
    assert!((bx - 0.0).abs() < 1e-4, "bx={bx}");
    assert!((by - 10.0).abs() < 1e-4, "by={by}");
}

#[test]
fn boundary_point_diagonal() {
    // px=10, py=10 → normalized direction (1/sqrt2, 1/sqrt2) → result at (10/sqrt2, 10/sqrt2)
    let (bx, by) = boundary_point(0.0, 0.0, 10.0, 10.0, 10.0);
    let expected = 10.0 / 2.0f32.sqrt();
    assert!((bx - expected).abs() < 1e-3, "bx={bx}");
    assert!((by - expected).abs() < 1e-3, "by={by}");
}

#[test]
fn boundary_point_coincident_returns_junction_center() {
    // px==jx, py==jy → degenerate, returns (jx, jy)
    let (bx, by) = boundary_point(5.0, 3.0, 10.0, 5.0, 3.0);
    assert!((bx - 5.0).abs() < 1e-4);
    assert!((by - 3.0).abs() < 1e-4);
}

// ---- dist ----

#[test]
fn dist_pythagorean() {
    assert!((dist((0.0, 0.0), (3.0, 4.0)) - 5.0).abs() < 1e-4);
}

#[test]
fn dist_zero_same_point() {
    assert_eq!(dist((1.0, 2.0), (1.0, 2.0)), 0.0);
}

#[test]
fn dist_negative_coords() {
    assert!((dist((-3.0, 0.0), (0.0, 4.0)) - 5.0).abs() < 1e-4);
}

// ---- lerp ----

#[test]
fn lerp_at_zero() {
    assert_eq!(lerp(2.0, 5.0, 0.0), 2.0);
}

#[test]
fn lerp_at_one() {
    assert_eq!(lerp(2.0, 5.0, 1.0), 5.0);
}

#[test]
fn lerp_at_half() {
    assert!((lerp(2.0, 5.0, 0.5) - 3.5).abs() < 1e-6);
}

#[test]
fn lerp_clamp_below_zero() {
    assert_eq!(lerp(2.0, 5.0, -1.0), 2.0);
}

#[test]
fn lerp_clamp_above_one() {
    assert_eq!(lerp(2.0, 5.0, 2.0), 5.0);
}

// ---- perp_right ----

#[test]
fn perp_right_east_gives_south_in_screen_coords() {
    // dx=1, dy=0 → perp_right = (-0/1, 1/1) = (0, 1)
    let (px, py) = perp_right(1.0, 0.0);
    assert!((px - 0.0).abs() < 1e-6, "px={px}");
    assert!((py - 1.0).abs() < 1e-6, "py={py}");
}

#[test]
fn perp_right_north_in_screen_gives_east() {
    // Going north in screen coords: dx=0, dy=-1 → (-(-1)/1, 0/1) = (1, 0)
    let (px, py) = perp_right(0.0, -1.0);
    assert!((px - 1.0).abs() < 1e-6, "px={px}");
    assert!((py - 0.0).abs() < 1e-6, "py={py}");
}

#[test]
fn perp_right_zero_vector_returns_default() {
    let (px, py) = perp_right(0.0, 0.0);
    assert_eq!((px, py), (1.0, 0.0));
}

#[test]
fn perp_right_result_is_unit_length() {
    let (px, py) = perp_right(3.0, 4.0);
    let len = (px * px + py * py).sqrt();
    assert!((len - 1.0).abs() < 1e-5, "len={len}");
}

// ---- lane_boundary_point ----

#[test]
fn lane_boundary_point_lane_zero() {
    // base=(0,0), perp=(1,0), idx=0, width=7.5 → offset=3.75 → (3.75, 0)
    let (x, y) = lane_boundary_point((0.0, 0.0), (1.0, 0.0), 0, 7.5);
    assert!((x - 3.75).abs() < 1e-4, "x={x}");
    assert!((y - 0.0).abs() < 1e-4, "y={y}");
}

#[test]
fn lane_boundary_point_lane_one() {
    // offset = 1.5 * 7.5 = 11.25
    let (x, y) = lane_boundary_point((0.0, 0.0), (1.0, 0.0), 1, 7.5);
    assert!((x - 11.25).abs() < 1e-4, "x={x}");
    assert!((y - 0.0).abs() < 1e-4, "y={y}");
}

// ---- foe_is_to_the_right ----
// Using screen/y-down coordinate system where +y is south.

#[test]
fn foe_is_to_the_right_true() {
    // Ego coming from south (entry=(0,10)), junction=(0,0), going north (-y).
    // Foe coming from east (entry=(10,0)), going west (-x).
    // In screen coords, going north (-y), right is east (+x) → foe from east IS to the right.
    let ego = Link {
        entry: (0.0, 10.0),
        junction_center: (0.0, 0.0),
        link_type: LinkType::Priority,
        foe_links: vec![],
        foe_internal_lane_ids: vec![],
        id: 0, lane_origin_id: 0, lane_destination_id: 0,
        via_internal_lane_id: 0, destination_road_id: 0,
    };
    let foe = FoeLink {
        id: 1,
        link_type: LinkType::Priority,
        entry: (10.0, 0.0),
    };
    assert!(foe_is_to_the_right(&ego, &foe));
}

#[test]
fn foe_is_to_the_right_false() {
    // Same ego (going north), but foe from west (entry=(-10,0)).
    // In screen coords, going north (-y), left is west → foe from west is to the LEFT.
    let ego = Link {
        entry: (0.0, 10.0),
        junction_center: (0.0, 0.0),
        link_type: LinkType::Priority,
        foe_links: vec![],
        foe_internal_lane_ids: vec![],
        id: 0, lane_origin_id: 0, lane_destination_id: 0,
        via_internal_lane_id: 0, destination_road_id: 0,
    };
    let foe = FoeLink {
        id: 1,
        link_type: LinkType::Priority,
        entry: (-10.0, 0.0),
    };
    assert!(!foe_is_to_the_right(&ego, &foe));
}

// ---- time_window_conflict ----

#[test]
fn twc_direct_window_overlap() {
    // ego=(5,7), foe=(4,6) with zero impatience → overlapping → true
    let result = time_window_conflict(5.0, 7.0, 4.0, 6.0, 10.0, 10.0, 3.0, 0.1, false, 0.0);
    assert!(result);
}

#[test]
fn twc_ego_well_after_foe_leaves() {
    // ego arrives 7s after foe leaves, look_ahead=0.1 → gap=7 >> 0.1 → false
    let result = time_window_conflict(10.0, 12.0, 1.0, 3.0, 10.0, 10.0, 3.0, 0.1, false, 0.0);
    assert!(!result);
}

#[test]
fn twc_gap_smaller_than_look_ahead() {
    // ego arrives 0.05s after foe leaves, look_ahead=0.1 → gap too small → true
    let result = time_window_conflict(3.05, 5.0, 1.0, 3.0, 10.0, 10.0, 3.0, 0.1, false, 0.0);
    assert!(result);
}

#[test]
fn twc_ego_entirely_before_foe() {
    // ego leaves well before foe arrives → false
    let result = time_window_conflict(1.0, 3.0, 10.0, 12.0, 10.0, 10.0, 3.0, 0.1, false, 0.0);
    assert!(!result);
}

#[test]
fn twc_impatience_shifts_foe_window() {
    // impatience=1 shifts foe window by look_ahead*2 forward.
    // If ego is just after foe with no impatience (no conflict), adding impatience
    // might keep it no-conflict or cause one depending on values.
    // Here: ego=10, foe=1.0-3.0, look_ahead=0.1, gap=7 with impatience=0 → no conflict
    // With impatience=1, foe_arrival_adj = lerp(1, 1.2, 1) = 1.2 → gap still 10-3.2=6.8 → no conflict
    let no_impatience = time_window_conflict(10.0, 12.0, 1.0, 3.0, 10.0, 10.0, 3.0, 0.1, false, 0.0);
    let with_impatience = time_window_conflict(10.0, 12.0, 1.0, 3.0, 10.0, 10.0, 3.0, 0.1, false, 1.0);
    // Both are false (gap large enough), impatience shift is small (0.2s)
    assert!(!no_impatience);
    assert!(!with_impatience);
}

#[test]
fn twc_foe_just_arrived_conflict() {
    // Windows exactly adjacent: ego(3,5), foe(1,3) → foe_leave_adj=3 == ego_arrival=3
    // The check: ego_arrival < foe_leave_adj → 3 < 3 is false
    // foe_leave_adj < ego_arrival → 3 < 3 is false
    // Falls through to true
    let result = time_window_conflict(3.0, 5.0, 1.0, 3.0, 10.0, 10.0, 3.0, 0.1, false, 0.0);
    assert!(result);
}

// ---- is_link_open ----

fn make_vehicle_on_road(id: u64) -> Vehicle {
    let origin = NodeIndex::new(0);
    let dest = NodeIndex::new(1);
    make_vehicle(id, origin, dest)
}

#[test]
fn is_link_open_already_in_internal_lane_always_true() {
    let link = make_default_link(0);
    let mut vehicle = make_vehicle_on_road(1);
    vehicle.current_lane = Some(LaneId::Internal(0, 0));
    let ego_data = make_approach_data(1.0, 2.0);
    let result = is_link_open(
        &link, &vehicle, &ego_data, &HashMap::new(), &HashMap::new(), &[], 0, 0.1, 1.0,
    );
    assert!(result);
}

#[test]
fn is_link_open_stop_sign_not_waited() {
    let mut link = make_default_link(0);
    link.link_type = LinkType::Stop;
    let mut vehicle = make_vehicle_on_road(1);
    vehicle.waiting_time = 0.0; // hasn't waited yet
    let ego_data = make_approach_data(1.0, 2.0);
    let result = is_link_open(
        &link, &vehicle, &ego_data, &HashMap::new(), &HashMap::new(), &[], 0, 0.1, 1.0,
    );
    assert!(!result);
}

#[test]
fn is_link_open_stop_sign_after_dwell() {
    let mut link = make_default_link(0);
    link.link_type = LinkType::Stop;
    let mut vehicle = make_vehicle_on_road(1);
    vehicle.waiting_time = 2.0; // > STOP_DWELL_TIME=1.0
    let ego_data = make_approach_data(1.0, 2.0);
    let result = is_link_open(
        &link, &vehicle, &ego_data, &HashMap::new(), &HashMap::new(), &[], 0, 0.1, 1.0,
    );
    assert!(result);
}

#[test]
fn is_link_open_foe_in_internal_lane_blocks() {
    let junction_id = 5u32;
    let foe_il_id = 99u32;
    let mut link = make_default_link(0);
    link.foe_internal_lane_ids = vec![foe_il_id];

    let vehicle = make_vehicle_on_road(1);
    let ego_data = make_approach_data(1.0, 2.0);

    // Put a dummy vehicle in the conflicting internal lane
    let mut vehicles_by_lane: HashMap<LaneId, Vec<usize>> = HashMap::new();
    vehicles_by_lane.insert(LaneId::Internal(junction_id, foe_il_id), vec![0]);

    let dummy = make_vehicle_on_road(99);
    let result = is_link_open(
        &link, &vehicle, &ego_data, &HashMap::new(), &vehicles_by_lane, &[dummy], junction_id, 0.1, 1.0,
    );
    assert!(!result);
}

#[test]
fn is_link_open_foe_internal_lane_empty_allows() {
    let junction_id = 5u32;
    let foe_il_id = 99u32;
    let mut link = make_default_link(0);
    link.foe_internal_lane_ids = vec![foe_il_id];

    let vehicle = make_vehicle_on_road(1);
    let ego_data = make_approach_data(1.0, 2.0);

    // Empty vehicle list for the conflicting lane
    let mut vehicles_by_lane: HashMap<LaneId, Vec<usize>> = HashMap::new();
    vehicles_by_lane.insert(LaneId::Internal(junction_id, foe_il_id), vec![]);

    let result = is_link_open(
        &link, &vehicle, &ego_data, &HashMap::new(), &vehicles_by_lane, &[], junction_id, 0.1, 1.0,
    );
    assert!(result);
}

#[test]
fn is_link_open_priority_no_foes() {
    let link = make_default_link(0); // Priority, no foe_links
    let vehicle = make_vehicle_on_road(1);
    let ego_data = make_approach_data(1.0, 2.0);
    let result = is_link_open(
        &link, &vehicle, &ego_data, &HashMap::new(), &HashMap::new(), &[], 0, 0.1, 1.0,
    );
    assert!(result);
}

#[test]
fn is_link_open_yield_to_priority_conflict_blocks() {
    // ego: Yield, foe: Priority, overlapping time windows → blocked
    let foe_link_id = 10u32;
    let mut link = make_default_link(0);
    link.link_type = LinkType::Yield;
    link.foe_links = vec![make_foe_link(foe_link_id, LinkType::Priority, (50.0, 0.0))];

    let mut vehicle = make_vehicle_on_road(1); // ego id=1
    vehicle.impatience = 0.0;
    let ego_data = make_approach_data(2.0, 4.0);

    // Foe with id=2 is approaching link 10, arrival=1, leave=3 (overlaps with ego arrival=2)
    let foe_vehicle = make_vehicle_on_road(2);
    let mut link_states: HashMap<u32, LinkState> = HashMap::new();
    let mut foe_state = LinkState::default();
    foe_state.approaching.insert(2, ApproachData {
        arrival_time: 1.0,
        leave_time: 3.0,
        arrival_speed: 10.0,
        leave_speed: 10.0,
        will_pass: true,
    });
    link_states.insert(foe_link_id, foe_state);

    let result = is_link_open(
        &link, &vehicle, &ego_data, &link_states, &HashMap::new(), &[foe_vehicle], 0, 0.1, 1.0,
    );
    assert!(!result, "expected blocked by yielding to priority foe");
}

#[test]
fn is_link_open_priority_ignores_yield_foe() {
    // ego: Priority, foe: Yield → must_yield=false → skipped, no blocking
    let foe_link_id = 10u32;
    let mut link = make_default_link(0);
    link.link_type = LinkType::Priority;
    link.foe_links = vec![make_foe_link(foe_link_id, LinkType::Yield, (50.0, 0.0))];

    let vehicle = make_vehicle_on_road(1);
    let ego_data = make_approach_data(2.0, 4.0);

    let foe_vehicle = make_vehicle_on_road(2);
    let mut link_states: HashMap<u32, LinkState> = HashMap::new();
    let mut foe_state = LinkState::default();
    foe_state.approaching.insert(2, ApproachData {
        arrival_time: 1.0, leave_time: 3.0,
        arrival_speed: 10.0, leave_speed: 10.0, will_pass: true,
    });
    link_states.insert(foe_link_id, foe_state);

    let result = is_link_open(
        &link, &vehicle, &ego_data, &link_states, &HashMap::new(), &[foe_vehicle], 0, 0.1, 1.0,
    );
    assert!(result, "Priority vehicle should not be blocked by Yield foe");
}

// ---- build_intersections ----

#[test]
fn build_intersections_populates_internal_lanes() {
    // The minimal straight map already calls build_intersections.
    // The junction (node 1) should have internal lanes.
    let map = make_minimal_straight_map();
    let jct = map.find_node(1).unwrap();
    let junction = &map.graph[jct];
    assert!(
        !junction.internal_lanes.is_empty(),
        "junction should have internal lanes after build_intersections"
    );
    // Each internal lane should have valid entry/exit
    for il in &junction.internal_lanes {
        assert!(il.length > 0.0, "internal lane length must be positive");
    }
}

#[test]
fn build_intersections_no_internal_on_isolated_node() {
    // A single node with no roads → no internal lanes built
    let mut map = Map::new();
    let _n = map.add_intersection(IntersectionKind::Intersection, 100.0, 100.0);
    build_intersections(&mut map);
    let ni = map.find_node(0).unwrap();
    assert!(
        map.graph[ni].internal_lanes.is_empty(),
        "isolated node should have no internal lanes"
    );
}

#[test]
fn build_intersections_populates_links_on_lanes() {
    // After building, road lanes should have links pointing through the junction
    let map = make_minimal_straight_map();
    // Find the hab→jct edge and check its lanes have links
    let hab = map.find_node(0).unwrap();
    let jct = map.find_node(1).unwrap();
    let edge = map.graph.find_edge(hab, jct).unwrap();
    let road = &map.graph[edge];
    assert!(!road.lanes.is_empty());
    assert!(
        road.lanes.iter().any(|l| !l.links.is_empty()),
        "road lanes should have links after build_intersections"
    );
}
