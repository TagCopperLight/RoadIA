use crate::map::intersection::{
    boundary_point, build_intersections, cross, dist, lane_boundary_point, on_segment, perp_right,
    segments_intersect, IntersectionKind,
};
use crate::map::model::Map;
use crate::test::make_minimal_straight_map;

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

#[test]
fn build_intersections_allows_u_turns_only_at_dead_ends() {
    // hab(0) — jct(1) — dead_end(2): the dead end needs a U-turn, the junction does not.
    let map = make_minimal_straight_map();
    let jct = map.find_node(1).unwrap();
    let work = map.find_node(2).unwrap();

    let into_dead_end = map.graph.find_edge(jct, work).unwrap();
    let back_road = map.graph[map.graph.find_edge(work, jct).unwrap()].id;
    let u_turns: Vec<_> = map.graph[into_dead_end].lanes[0]
        .links
        .iter()
        .filter(|link| link.destination_road_id == back_road)
        .collect();
    assert_eq!(u_turns.len(), 1, "dead end should offer a U-turn");

    let hab = map.find_node(0).unwrap();
    let into_jct = map.graph.find_edge(hab, jct).unwrap();
    let back_to_hab = map.graph[map.graph.find_edge(jct, hab).unwrap()].id;
    assert!(
        map.graph[into_jct].lanes[0]
            .links
            .iter()
            .all(|link| link.destination_road_id != back_to_hab),
        "no U-turn where the road continues"
    );
}
