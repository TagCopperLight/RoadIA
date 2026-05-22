use crate::map::intersection::{build_intersections, IntersectionKind};
use crate::map::model::Map;
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::simulation::vehicle::{LaneId, TripRequest, Vehicle, VehicleKind, VehicleSpec, VehicleState, VehicleType};
use crate::test::make_sim_config;

fn make_two_lane_map() -> Map {
    let mut map = Map::new();
    map.add_intersection(IntersectionKind::Habitation,   0.0,    0.0);
    map.add_intersection(IntersectionKind::Intersection, 1000.0, 0.0);
    map.add_intersection(IntersectionKind::Workplace,    2000.0, 0.0);
    map.add_two_way_road(0, 1, 2, 40.0, 1000.0);
    map.add_two_way_road(1, 2, 2, 40.0, 1000.0);
    build_intersections(&mut map);
    map
}

#[test]
fn overtake_slow_leader() {
    let map = make_two_lane_map();
    let hab  = map.find_node(0).unwrap();
    let work = map.find_node(2).unwrap();

    // Slow leader departs first (vehicle index 0 processed first by departure queue)
    let slow_spec = VehicleSpec::new(VehicleKind::Car, 8.0, 2.0, 3.0, 1.0, 5.0);
    let mut slow = Vehicle::new(
        0,
        slow_spec,
        TripRequest { origin: hab, destination: work, departure_time: 0.0 },
        VehicleType::Essence,
    );
    slow.update_path(&map);

    let fast_spec = VehicleSpec::new(VehicleKind::Car, 40.0, 4.0, 3.0, 1.0, 5.0);
    let mut fast = Vehicle::new(
        1,
        fast_spec,
        TripRequest { origin: hab, destination: work, departure_time: 0.0 },
        VehicleType::Essence,
    );
    fast.update_path(&map);

    // Without LC2013 overtaking: fast vehicle limited to ~8 m/s → ~250s to cover 2000m.
    // With LC2013 overtaking: fast vehicle switches to lane 1 → ~50s + overhead.
    let config = make_sim_config(map, 200.0);
    let mut engine = SimulationEngine::new(config, vec![slow, fast]);
    engine.run();

    assert_eq!(
        engine.vehicles[1].state,
        VehicleState::Arrived,
        "fast vehicle should have arrived within 200s, requiring successful overtake"
    );
}

#[test]
fn keep_right_in_clear_left_lane() {
    let map = make_two_lane_map();
    let hab  = map.find_node(0).unwrap();
    let jct  = map.find_node(1).unwrap();
    let work = map.find_node(2).unwrap();
    let first_edge = map.graph.find_edge(hab, jct).unwrap();
    let lane1 = LaneId::Normal(first_edge, 1);

    let spec = VehicleSpec::new(VehicleKind::Car, 40.0, 4.0, 3.0, 1.0, 5.0);
    let mut v = Vehicle::new(
        0,
        spec,
        TripRequest { origin: hab, destination: work, departure_time: 0.0 },
        VehicleType::Essence,
    );
    v.update_path(&map);
    // Pre-place in lane 1
    v.state = VehicleState::OnRoad;
    v.current_lane = Some(lane1);
    v.position_on_lane = 10.0;
    v.velocity = 5.0;
    v.previous_velocity = 5.0;

    let config = make_sim_config(map, 30.0);
    let mut engine = SimulationEngine::new(config, vec![v]);
    engine.insert_vehicle_into_lane(lane1, 0);

    let end_time = engine.current_time + 10.0;
    while engine.current_time < end_time {
        engine.step();
        engine.current_time += engine.config.time_step;
    }

    let in_lane_0 = matches!(engine.vehicles[0].current_lane, Some(LaneId::Normal(_, 0)));
    assert!(in_lane_0, "vehicle should have kept right to lane 0");
}
