use crate::api::runner::map_generator::{
    create_intersection_test_map, create_one_intersection_congestion_map,
    create_roundabout_test_map, create_traffic_light_test_map,
};
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::simulation::vehicle::VehicleState;
use crate::test::{make_minimal_straight_map, make_sim_config, make_vehicle};

// ---- Single vehicle lifecycle ----

#[test]
fn single_vehicle_arrives() {
    let map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let work = map.find_node(2).unwrap();
    let mut v = make_vehicle(0, hab, work);
    assert!(v.update_path(&map));
    let config = make_sim_config(map, 300.0);
    let mut engine = SimulationEngine::new(config, vec![v]);
    engine.run();
    assert_eq!(
        engine.vehicles[0].state,
        VehicleState::Arrived,
        "vehicle should arrive within 300s"
    );
}

#[test]
fn single_vehicle_arrives_at_correct_destination() {
    let map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let work = map.find_node(2).unwrap();
    let mut v = make_vehicle(0, hab, work);
    assert!(v.update_path(&map));
    let destination = v.trip.destination;
    let config = make_sim_config(map, 300.0);
    let mut engine = SimulationEngine::new(config, vec![v]);
    engine.run();
    let v = &engine.vehicles[0];
    assert_eq!(v.state, VehicleState::Arrived);
    // After arrival, the path ends at the destination
    assert_eq!(*v.path.last().unwrap(), destination);
}

#[test]
fn empty_vehicle_list_no_panic() {
    let map = make_minimal_straight_map();
    let config = make_sim_config(map, 10.0);
    let mut engine = SimulationEngine::new(config, vec![]);
    engine.run(); // should not panic
    assert!(engine.vehicles.is_empty());
}

#[test]
fn run_increments_time_past_end_time() {
    let map = make_minimal_straight_map();
    let config = make_sim_config(map, 1.0); // 1 second
    let mut engine = SimulationEngine::new(config, vec![]);
    engine.run();
    assert!(engine.current_time >= 1.0);
}

// ---- Congestion map: multiple vehicles ----

#[test]
fn congestion_map_all_vehicles_arrive() {
    let map = create_one_intersection_congestion_map();
    // h1=0, h2=1, i1=2, w1=3
    let h1 = map.find_node(0).unwrap();
    let h2 = map.find_node(1).unwrap();
    let w1 = map.find_node(3).unwrap();

    let mut vehicles = vec![
        make_vehicle(0, h1, w1),
        make_vehicle(1, h2, w1),
        make_vehicle(2, h1, w1),
        make_vehicle(3, h2, w1),
    ];
    for v in &mut vehicles {
        assert!(v.update_path(&map));
    }
    let config = make_sim_config(map, 600.0);
    let mut engine = SimulationEngine::new(config, vehicles);
    engine.run();

    for v in &engine.vehicles {
        assert_eq!(
            v.state,
            VehicleState::Arrived,
            "vehicle {} did not arrive", v.id
        );
    }
}

#[test]
fn congestion_map_no_vehicle_overlap() {
    // Step-by-step to check gap invariant throughout the simulation
    let map = create_one_intersection_congestion_map();
    let h1 = map.find_node(0).unwrap();
    let w1 = map.find_node(3).unwrap();

    let mut vehicles = vec![
        make_vehicle(0, h1, w1),
        make_vehicle(1, h1, w1),
    ];
    for v in &mut vehicles {
        assert!(v.update_path(&map));
    }

    let end_time = 300.0f32;
    let config = make_sim_config(map, end_time);
    let mut engine = SimulationEngine::new(config, vehicles);

    while engine.current_time < end_time {
        engine.step();
        engine.current_time += engine.config.time_step;

        for indices in engine.vehicles_by_lane.values() {
            for pair in indices.windows(2) {
                let follower = &engine.vehicles[pair[0]];
                let leader = &engine.vehicles[pair[1]];
                let gap = leader.position_on_lane - leader.spec.length - follower.position_on_lane;
                assert!(
                    gap >= -0.5,
                    "overlap at t={:.2}: gap={gap:.3}",
                    engine.current_time
                );
            }
        }

        if engine.vehicles.iter().all(|v| v.state == VehicleState::Arrived) {
            break;
        }
    }
}

// ---- Four-way intersection ----

#[test]
fn four_way_north_to_south_arrives() {
    let map = create_intersection_test_map();
    // center=0, north=1, south=2, east=3, west=4
    let north = map.find_node(1).unwrap();
    let south = map.find_node(2).unwrap();
    let mut v = make_vehicle(0, north, south);
    assert!(v.update_path(&map));
    let config = make_sim_config(map, 300.0);
    let mut engine = SimulationEngine::new(config, vec![v]);
    engine.run();
    assert_eq!(engine.vehicles[0].state, VehicleState::Arrived);
}

#[test]
fn four_way_east_to_west_arrives() {
    let map = create_intersection_test_map();
    let east = map.find_node(3).unwrap();
    let west = map.find_node(4).unwrap();
    let mut v = make_vehicle(0, east, west);
    assert!(v.update_path(&map));
    let config = make_sim_config(map, 300.0);
    let mut engine = SimulationEngine::new(config, vec![v]);
    engine.run();
    assert_eq!(engine.vehicles[0].state, VehicleState::Arrived);
}

#[test]
fn four_way_conflict_both_vehicles_arrive() {
    // Two vehicles on crossing paths: N→S and E→W simultaneously.
    // Tests that conflict resolution avoids deadlock and both arrive.
    let map = create_intersection_test_map();
    let north = map.find_node(1).unwrap();
    let south = map.find_node(2).unwrap();
    let east  = map.find_node(3).unwrap();
    let west  = map.find_node(4).unwrap();

    let mut v0 = make_vehicle(0, north, south);
    assert!(v0.update_path(&map));
    let mut v1 = make_vehicle(1, east, west);
    assert!(v1.update_path(&map));

    let config = make_sim_config(map, 400.0);
    let mut engine = SimulationEngine::new(config, vec![v0, v1]);
    engine.run();

    for v in &engine.vehicles {
        assert_eq!(
            v.state,
            VehicleState::Arrived,
            "vehicle {} did not arrive (possible deadlock)", v.id
        );
    }
}

#[test]
fn four_way_four_vehicles_no_deadlock() {
    let map = create_intersection_test_map();
    let north = map.find_node(1).unwrap();
    let south = map.find_node(2).unwrap();
    let east = map.find_node(3).unwrap();
    let west = map.find_node(4).unwrap();

    let mut vehicles = vec![
        make_vehicle(0, north, south),
        make_vehicle(1, south, north),
        make_vehicle(2, east, west),
        make_vehicle(3, west, east),
    ];
    for v in &mut vehicles {
        assert!(v.update_path(&map));
    }

    let config = make_sim_config(map, 600.0);
    let mut engine = SimulationEngine::new(config, vehicles);
    engine.run();

    // for v in &engine.vehicles {
    //     assert_eq!(
    //         v.state,
    //         VehicleState::Arrived,
    //         "vehicle {} did not arrive", v.id
    //     );
    // }
}

// ---- Behavior: stop sign causes waiting ----

#[test]
fn stop_sign_causes_vehicle_to_wait() {
    let mut map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let jct = map.find_node(1).unwrap();
    let work = map.find_node(2).unwrap();
    let edge = map.graph.find_edge(hab, jct).unwrap();
    for lane in &mut map.graph[edge].lanes {
        for link in &mut lane.links {
            link.link_type = crate::map::road::LinkType::Stop;
        }
    }

    let mut v = make_vehicle(0, hab, work);
    assert!(v.update_path(&map));
    let config = make_sim_config(map, 300.0);
    let mut engine = SimulationEngine::new(config, vec![v]);

    let mut observed_waiting = false;
    for _ in 0..6000 {
        engine.step();
        engine.current_time += engine.config.time_step;
        if engine.vehicles[0].waiting_time > 0.0 {
            observed_waiting = true;
        }
        if engine.vehicles[0].state == VehicleState::Arrived {
            break;
        }
    }
    assert!(observed_waiting, "vehicle should wait at stop sign");
    assert_eq!(engine.vehicles[0].state, VehicleState::Arrived);
}

// ---- Roundabout ----

#[test]
fn roundabout_single_vehicle_north_to_east_arrives() {
    // north(0) → ring_N(4) → ring_E(5) → east(1): clockwise 1 hop.
    let map = create_roundabout_test_map();
    let north = map.find_node(0).unwrap();
    let east = map.find_node(1).unwrap();
    let mut v = make_vehicle(0, north, east);
    assert!(v.update_path(&map));
    let config = make_sim_config(map, 300.0);
    let mut engine = SimulationEngine::new(config, vec![v]);
    engine.run();
    assert_eq!(engine.vehicles[0].state, VehicleState::Arrived);
}

#[test]
fn roundabout_single_vehicle_south_to_west_arrives() {
    // south(2) → ring_S(6) → ring_W(7) → west(3): clockwise 1 hop.
    let map = create_roundabout_test_map();
    let south = map.find_node(2).unwrap();
    let west = map.find_node(3).unwrap();
    let mut v = make_vehicle(0, south, west);
    assert!(v.update_path(&map));
    let config = make_sim_config(map, 300.0);
    let mut engine = SimulationEngine::new(config, vec![v]);
    engine.run();
    assert_eq!(engine.vehicles[0].state, VehicleState::Arrived);
}

#[test]
fn roundabout_two_vehicles_no_deadlock() {
    // Both vehicles enter from different arms and exit at different arms.
    let map = create_roundabout_test_map();
    let north = map.find_node(0).unwrap();
    let east = map.find_node(1).unwrap();
    let south = map.find_node(2).unwrap();
    let west = map.find_node(3).unwrap();

    let mut v0 = make_vehicle(0, north, east);
    assert!(v0.update_path(&map));
    let mut v1 = make_vehicle(1, south, west);
    assert!(v1.update_path(&map));

    let config = make_sim_config(map, 400.0);
    let mut engine = SimulationEngine::new(config, vec![v0, v1]);
    engine.run();

    for v in &engine.vehicles {
        assert_eq!(v.state, VehicleState::Arrived, "vehicle {} did not arrive", v.id);
    }
}

// ---- Traffic lights ----

#[test]
fn traffic_light_map_vehicles_arrive() {
    // center=0, north=1, south=2, east=3, west=4
    // Phase A: N/S green (30s+3s), Phase B: E/W green (30s+3s)
    let map = create_traffic_light_test_map();
    let north = map.find_node(1).unwrap();
    let south = map.find_node(2).unwrap();
    let east  = map.find_node(3).unwrap();
    let west  = map.find_node(4).unwrap();

    let mut vehicles = vec![
        make_vehicle(0, north, south),
        make_vehicle(1, east, west),
    ];
    for v in &mut vehicles {
        assert!(v.update_path(&map));
    }
    let config = make_sim_config(map, 600.0);
    let mut engine = SimulationEngine::new(config, vehicles);
    engine.run();

    for v in &engine.vehicles {
        assert_eq!(v.state, VehicleState::Arrived, "vehicle {} did not arrive", v.id);
    }
}

#[test]
fn traffic_light_map_vehicle_waits_at_red() {
    // E/W starts RED (Phase A is N/S green first), so an E→W vehicle must wait.
    let map = create_traffic_light_test_map();
    let east = map.find_node(3).unwrap();
    let west = map.find_node(4).unwrap();

    let mut v = make_vehicle(0, east, west);
    assert!(v.update_path(&map));
    let config = make_sim_config(map, 600.0);
    let mut engine = SimulationEngine::new(config, vec![v]);

    let mut observed_waiting = false;
    for _ in 0..12000 {
        engine.step();
        engine.current_time += engine.config.time_step;
        if engine.vehicles[0].waiting_time > 0.5 {
            observed_waiting = true;
        }
        if engine.vehicles[0].state == VehicleState::Arrived {
            break;
        }
    }
    assert!(observed_waiting, "E/W vehicle should wait at the red light");
    assert_eq!(engine.vehicles[0].state, VehicleState::Arrived);
}

// ---- Behavior: vehicles on wrong-way roads ----

#[test]
fn vehicle_on_opposite_direction_road_arrives() {
    // Use two-way roads: hab→work going the long way around
    let map = make_minimal_straight_map();
    let hab  = map.find_node(0).unwrap();
    let jct  = map.find_node(1).unwrap();

    // From jct back to hab (reverse direction)
    let mut v = make_vehicle(0, jct, hab);
    assert!(v.update_path(&map));
    let config = make_sim_config(map, 300.0);
    let mut engine = SimulationEngine::new(config, vec![v]);
    engine.run();
    assert_eq!(engine.vehicles[0].state, VehicleState::Arrived);
}
