use petgraph::graph::NodeIndex;

use crate::map::intersection::IntersectionKind;
use crate::map::model::Map;
use crate::simulation::vehicle::{fastest_path, LaneId, Vehicle, VehicleState};
use crate::test::{make_minimal_straight_map, make_standard_spec, make_vehicle};

fn make_vehicle_with_prev_velocity(prev_v: f32) -> Vehicle {
    let origin = NodeIndex::new(0);
    let dest = NodeIndex::new(1);
    let mut v = make_vehicle(0, origin, dest);
    v.previous_velocity = prev_v;
    v
}

// ---- Vehicle::new initial state ----

#[test]
fn vehicle_new_initial_state() {
    let origin = NodeIndex::new(0);
    let dest = NodeIndex::new(1);
    let v = make_vehicle(42, origin, dest);
    assert_eq!(v.id, 42);
    assert_eq!(v.state, VehicleState::WaitingToDepart);
    assert_eq!(v.velocity, 0.0);
    assert_eq!(v.previous_velocity, 0.0);
    assert_eq!(v.position_on_lane, 0.0);
    assert!(v.path.is_empty());
    assert_eq!(v.path_index, 0);
    assert!(v.current_lane.is_none());
    assert_eq!(v.waiting_time, 0.0);
    assert_eq!(v.impatience, 0.0);
}

// ---- compute_acceleration (IDM) ----

#[test]
fn idm_free_road_at_desired_speed() {
    // previous_velocity == desired_velocity → free_road_acc = 0
    // no leader → interaction term = 0
    let mut v = make_vehicle_with_prev_velocity(20.0);
    v.spec = make_standard_spec(); // a=4, d=3, reaction=1
    let accel = v.compute_acceleration(20.0, 2.0, f32::INFINITY, 0.0);
    assert!(accel.abs() < 0.01, "expected ~0, got {accel}");
}

#[test]
fn idm_free_road_from_rest() {
    // previous_velocity=0 → free_road_acc = a_max
    // no leader → interaction = 0
    let mut v = make_vehicle_with_prev_velocity(0.0);
    v.spec = make_standard_spec();
    let accel = v.compute_acceleration(20.0, 2.0, f32::INFINITY, 0.0);
    assert!((accel - 4.0).abs() < 0.01, "expected ~4.0, got {accel}");
}

#[test]
fn idm_braking_for_close_vehicle() {
    // Leader very close and stopped → strong deceleration
    let mut v = make_vehicle_with_prev_velocity(15.0);
    v.spec = make_standard_spec();
    let accel = v.compute_acceleration(15.0, 2.0, 3.0, 0.0);
    assert!(accel < -1.0, "expected strong braking, got {accel}");
}

#[test]
fn idm_zero_ahead_distance_returns_max_decel() {
    // vehicle_ahead_distance <= 0 → emergency brake
    let mut v = make_vehicle_with_prev_velocity(15.0);
    v.spec = make_standard_spec();
    let accel = v.compute_acceleration(15.0, 2.0, -1.0, 0.0);
    assert_eq!(accel, -v.spec.comfortable_deceleration);
}

#[test]
fn idm_zero_minimum_gap_no_panic() {
    // minimum_gap=0 should be clamped to 0.1 internally, not panic
    let mut v = make_vehicle_with_prev_velocity(10.0);
    v.spec = make_standard_spec();
    let accel = v.compute_acceleration(20.0, 0.0, 50.0, 0.0);
    assert!(accel.is_finite());
}

#[test]
fn idm_following_at_safe_distance_small_positive_accel() {
    // Leader ahead at comfortable distance, same speed → near-zero accel
    let mut v = make_vehicle_with_prev_velocity(10.0);
    v.spec = make_standard_spec();
    // s_desired ≈ 2 + 10*1 + 0 = 12m, add extra buffer → accel near 0
    let accel = v.compute_acceleration(10.0, 2.0, 30.0, 10.0);
    // Should be moderate — not strongly braking, not strongly accelerating
    assert!(accel > -3.0 && accel < 4.0, "accel out of range: {accel}");
}

#[test]
fn idm_half_desired_speed_free_road() {
    // At v = 0.5 * v_des, free_road_acc = a_max * (1 - 0.5^4) = 4 * 0.9375 = 3.75
    let mut v = make_vehicle_with_prev_velocity(10.0);
    v.spec = make_standard_spec();
    let accel = v.compute_acceleration(20.0, 2.0, f32::INFINITY, 0.0);
    assert!((accel - 3.75).abs() < 0.01, "expected 3.75, got {accel}");
}

// ---- fastest_path ----

#[test]
fn fastest_path_direct_route() {
    let map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let work = map.find_node(2).unwrap();
    let path = fastest_path(&map, hab, work).expect("path should exist");
    assert_eq!(path.len(), 3);
    assert_eq!(path[0], hab);
    assert_eq!(path[2], work);
}

#[test]
fn fastest_path_prefers_high_speed_road() {
    // Build a map where:
    //   Route A: O→D direct, dist=500, speed=10  (cost=50)
    //   Route B: O→W→D via waypoint, dist=250+350=600, speed=40 (cost=6.25+8.75=15)
    // A* should prefer route B
    let mut map = Map::new();
    let origin_id = map.add_intersection(IntersectionKind::Habitation, 0.0, 0.0);
    let dest_id = map.add_intersection(IntersectionKind::Workplace, 500.0, 0.0);
    let waypoint_id = map.add_intersection(IntersectionKind::Intersection, 200.0, 100.0);
    map.add_road(origin_id, dest_id, 1, 10.0, 500.0); // slow direct
    map.add_road(origin_id, waypoint_id, 1, 40.0, 250.0);
    map.add_road(waypoint_id, dest_id, 1, 40.0, 350.0);

    let origin = map.find_node(origin_id).unwrap();
    let dest = map.find_node(dest_id).unwrap();
    let waypoint = map.find_node(waypoint_id).unwrap();

    let path = fastest_path(&map, origin, dest).expect("path should exist");
    assert!(
        path.contains(&waypoint),
        "expected route via waypoint (faster), got path of len {}",
        path.len()
    );
}

#[test]
fn fastest_path_no_path_returns_none() {
    let mut map = Map::new();
    let a_id = map.add_intersection(IntersectionKind::Habitation, 0.0, 0.0);
    let b_id = map.add_intersection(IntersectionKind::Workplace, 100.0, 0.0);
    // No road between them
    let a = map.find_node(a_id).unwrap();
    let b = map.find_node(b_id).unwrap();
    assert!(fastest_path(&map, a, b).is_none());
}

// ---- Vehicle::update_path ----

#[test]
fn update_path_populates_path() {
    let map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let work = map.find_node(2).unwrap();
    let mut v = make_vehicle(0, hab, work);
    assert!(v.path.is_empty());
    v.update_path(&map);
    assert!(!v.path.is_empty());
    assert_eq!(v.path_index, 0);
    assert_eq!(v.path[0], hab);
    assert_eq!(*v.path.last().unwrap(), work);
}

// ---- Vehicle::get_current_node / get_next_node ----

#[test]
fn get_current_node_returns_path_at_index() {
    let map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let work = map.find_node(2).unwrap();
    let mut v = make_vehicle(0, hab, work);
    v.update_path(&map);
    assert_eq!(v.get_current_node(), v.path[0]);
}

#[test]
fn get_next_node_returns_path_at_index_plus_one() {
    let map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let work = map.find_node(2).unwrap();
    let mut v = make_vehicle(0, hab, work);
    v.update_path(&map);
    assert_eq!(v.get_next_node(), v.path[1]);
}

// ---- Vehicle::get_coordinates ----

#[test]
fn get_coordinates_waiting_to_depart_returns_origin_coords() {
    let map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let work = map.find_node(2).unwrap();
    let mut v = make_vehicle(0, hab, work);
    v.update_path(&map);
    // WaitingToDepart → should return origin intersection coords
    let coords = v.get_coordinates(&map);
    // hab is at (0, 0)
    assert!((coords.x - 0.0).abs() < 1e-3);
    assert!((coords.y - 0.0).abs() < 1e-3);
}

#[test]
fn get_coordinates_on_road_interpolates() {
    let map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let jct = map.find_node(1).unwrap();
    let work = map.find_node(2).unwrap();
    let edge = map.graph.find_edge(hab, jct).unwrap();

    let mut v = make_vehicle(0, hab, work);
    v.update_path(&map);
    v.state = VehicleState::OnRoad;
    v.current_lane = Some(LaneId::Normal(edge, 0));
    v.position_on_lane = 250.0; // midpoint of 500m road

    let coords = v.get_coordinates(&map);
    // hab=(0,0), jct=(500,0) → midpoint ≈ (250 + lane_offset, 0)
    // with lane_idx=0, offset = 0.5 * 7.5 = 3.75 in the perpendicular direction
    // road is horizontal so perp is vertical → coords.x ≈ 250
    assert!((coords.x - 250.0).abs() < 1.0, "x={}", coords.x);
}
