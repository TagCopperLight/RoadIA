use crate::api::runner::map_generator::{
    create_intersection_test_map, create_multilane_test_map, create_roundabout_test_map,
    create_traffic_light_test_map,
};
use crate::map::model::Map;
use crate::map::road::LinkType;
use crate::simulation::engine::network::Network;
use crate::simulation::engine::{time_windows_conflict, ApproachData, SimulationEngine, Simulation};
use crate::simulation::vehicle::{LaneId, VehicleState};
use crate::test::{link_between, make_sim_config, make_trip, run_checked};

// Intersection test map: center=0, north=1 (top), south=2, east=3, west=4.
const CENTER: u32 = 0;
const NORTH: u32 = 1;
const SOUTH: u32 = 2;
const EAST: u32 = 3;
const WEST: u32 = 4;

fn window(arrival: f32, leave: f32) -> ApproachData {
    ApproachData { arrival_time: arrival, leave_time: leave }
}

// ---- time windows ----

#[test]
fn overlapping_windows_conflict() {
    assert!(time_windows_conflict(&window(5.0, 7.0), &window(4.0, 6.0), 0.5));
}

#[test]
fn windows_closer_than_the_gap_conflict() {
    assert!(time_windows_conflict(&window(3.2, 5.0), &window(1.0, 3.0), 0.5));
    assert!(time_windows_conflict(&window(1.0, 3.0), &window(3.2, 5.0), 0.5));
}

#[test]
fn separated_windows_do_not_conflict() {
    assert!(!time_windows_conflict(&window(10.0, 12.0), &window(1.0, 3.0), 0.5));
    assert!(!time_windows_conflict(&window(1.0, 3.0), &window(10.0, 12.0), 0.5));
}

// ---- who yields to whom ----

fn assert_one_of_each_foe_pair_yields(map: &Map) {
    let network = Network::build(map);
    let mut pairs = 0;
    for id in 0..network.link_id_bound() as u32 {
        let Some(link) = network.link(id) else { continue };
        for foe in &link.link.foe_links {
            let foe = network.link(foe.id).unwrap();
            assert_ne!(
                link.must_yield_to(foe),
                foe.must_yield_to(link),
                "exactly one of links {} and {} must yield",
                link.link.id,
                foe.link.id
            );
            pairs += 1;
        }
    }
    assert!(pairs > 0);
}

#[test]
fn exactly_one_vehicle_of_each_conflict_yields() {
    assert_one_of_each_foe_pair_yields(&create_intersection_test_map());
    assert_one_of_each_foe_pair_yields(&create_multilane_test_map());
    assert_one_of_each_foe_pair_yields(&create_roundabout_test_map());
}

#[test]
fn priority_to_the_right() {
    let map = create_intersection_test_map();
    let network = Network::build(&map);
    // Driving north (up the screen), traffic coming from the east is on the right.
    let northbound = network.link(link_between(&map, SOUTH, CENTER, NORTH)).unwrap();
    let westbound = network.link(link_between(&map, EAST, CENTER, WEST)).unwrap();
    assert!(northbound.link.foe_links.iter().any(|foe| foe.id == westbound.link.id));
    assert!(northbound.must_yield_to(westbound));
    assert!(!westbound.must_yield_to(northbound));
}

#[test]
fn left_turn_yields_to_oncoming_traffic() {
    let map = create_intersection_test_map();
    let network = Network::build(&map);
    let turning_left = network.link(link_between(&map, SOUTH, CENTER, WEST)).unwrap();
    let oncoming = network.link(link_between(&map, NORTH, CENTER, SOUTH)).unwrap();
    assert!(turning_left.link.foe_links.iter().any(|foe| foe.id == oncoming.link.id));
    assert!(turning_left.must_yield_to(oncoming));
    assert!(!oncoming.must_yield_to(turning_left));
}

#[test]
fn minor_road_yields_to_major_road() {
    let mut map = create_intersection_test_map();
    let minor = link_between(&map, SOUTH, CENTER, NORTH);
    for road in map.graph.edge_weights_mut() {
        for link in road.lanes.iter_mut().flat_map(|lane| &mut lane.links) {
            if link.id == minor {
                link.link_type = LinkType::Yield;
            }
        }
    }
    let network = Network::build(&map);
    let minor = network.link(minor).unwrap();
    // The vehicle from the west would normally yield (the south one is on its right).
    let major = network.link(link_between(&map, WEST, CENTER, EAST)).unwrap();
    assert!(minor.must_yield_to(major));
    assert!(!major.must_yield_to(minor));
}

// ---- whole junctions ----

#[test]
fn four_full_approaches_do_not_deadlock() {
    // Every approach has a queue and every vehicle has a foe on its right: with plain
    // priority to the right, nobody may ever go.
    let map = create_intersection_test_map();
    let routes = [(NORTH, SOUTH), (SOUTH, NORTH), (EAST, WEST), (WEST, EAST)];
    let vehicles = (0..16)
        .map(|i| {
            let (from, to) = routes[i % 4];
            make_trip(&map, i as u64, from, to, 0.0)
        })
        .collect();
    let mut engine = SimulationEngine::new(make_sim_config(map, 600.0), vehicles);
    assert!(run_checked(&mut engine, 600.0), "all vehicles should cross the junction");
}

#[test]
fn turning_traffic_from_all_sides_does_not_deadlock() {
    let map = create_intersection_test_map();
    let arms = [NORTH, EAST, SOUTH, WEST];
    let mut vehicles = Vec::new();
    for (i, &from) in arms.iter().enumerate() {
        for (j, &to) in arms.iter().enumerate() {
            if i != j {
                let id = vehicles.len() as u64;
                vehicles.push(make_trip(&map, id, from, to, (id % 3) as f32));
            }
        }
    }
    let mut engine = SimulationEngine::new(make_sim_config(map, 600.0), vehicles);
    assert!(run_checked(&mut engine, 600.0), "all vehicles should cross the junction");
}

#[test]
fn roundabout_handles_traffic_from_every_arm() {
    let map = create_roundabout_test_map();
    // north=0, east=1, south=2, west=3 (habitations and workplaces around the ring).
    let trips = [(0, 1), (0, 3), (2, 1), (2, 3)];
    let vehicles = (0..24)
        .map(|i| {
            let (from, to) = trips[i % trips.len()];
            make_trip(&map, i as u64, from, to, (i / trips.len()) as f32)
        })
        .collect();
    let mut engine = SimulationEngine::new(make_sim_config(map, 900.0), vehicles);
    assert!(run_checked(&mut engine, 900.0), "all vehicles should go through the roundabout");
}

#[test]
fn stop_sign_vehicle_stops_then_yields_to_priority_traffic() {
    let mut map = create_intersection_test_map();
    let stop = link_between(&map, WEST, CENTER, EAST);
    for road in map.graph.edge_weights_mut() {
        for link in road.lanes.iter_mut().flat_map(|lane| &mut lane.links) {
            if link.id == stop {
                link.link_type = LinkType::Stop;
            }
            for foe in &mut link.foe_links {
                if foe.id == stop {
                    foe.link_type = LinkType::Stop;
                }
            }
        }
    }
    let center = map.find_node(CENTER).unwrap();
    let west = map.find_node(WEST).unwrap();
    let approach = map.graph.find_edge(west, center).unwrap();

    // The stopping vehicle arrives first, then a stream of priority vehicles.
    let mut vehicles = vec![make_trip(&map, 0, WEST, EAST, 0.0)];
    for i in 1..6 {
        vehicles.push(make_trip(&map, i, NORTH, SOUTH, 12.0 + 2.0 * i as f32));
    }
    let mut engine = SimulationEngine::new(make_sim_config(map, 600.0), vehicles);

    let mut stopped_at_line = false;
    while engine.current_time < 600.0 {
        engine.step();
        engine.current_time += engine.config.time_step;
        let v = &engine.vehicles[0];
        if v.current_lane == Some(LaneId::Normal(approach, 0)) && v.velocity < 0.1 && v.position_on_lane > 480.0 {
            stopped_at_line = true;
        }
        if engine.vehicles.iter().all(|v| v.state == VehicleState::Arrived) {
            break;
        }
    }
    assert!(stopped_at_line, "the vehicle should stop at the stop line");
    assert!(engine.vehicles.iter().all(|v| v.state == VehicleState::Arrived));
}

#[test]
fn red_light_holds_vehicles_until_green() {
    // Phase 0 (30 s green + 3 s yellow) is north/south; east/west waits for phase 1.
    let map = create_traffic_light_test_map();
    let vehicles = vec![make_trip(&map, 0, EAST, WEST, 0.0)];
    let mut engine = SimulationEngine::new(make_sim_config(map, 300.0), vehicles);

    let mut entered_at = None;
    while engine.current_time < 300.0 && entered_at.is_none() {
        engine.step();
        engine.current_time += engine.config.time_step;
        if matches!(engine.vehicles[0].current_lane, Some(LaneId::Internal(..))) {
            entered_at = Some(engine.current_time);
        }
    }
    let entered_at = entered_at.expect("vehicle should cross on green");
    assert!(entered_at >= 33.0, "vehicle crossed on red at t={entered_at}");
}

#[test]
fn vehicle_keeps_the_junction_clear_when_its_exit_is_full() {
    let map = create_intersection_test_map();
    let center = map.find_node(CENTER).unwrap();
    let west = map.find_node(WEST).unwrap();
    let east = map.find_node(EAST).unwrap();
    let approach = map.graph.find_edge(west, center).unwrap();
    let exit = map.graph.find_edge(center, east).unwrap();

    // A broken-down vehicle sits at the start of the exit road.
    let mut blocker = make_trip(&map, 0, CENTER, EAST, f32::MAX);
    blocker.spec.max_speed = 0.0;
    blocker.state = VehicleState::OnRoad;
    blocker.position_on_lane = 4.0;
    blocker.current_lane = Some(LaneId::Normal(exit, 0));

    let mut vehicle = make_trip(&map, 1, WEST, EAST, f32::MAX);
    vehicle.state = VehicleState::OnRoad;
    vehicle.position_on_lane = 450.0;
    vehicle.velocity = 10.0;
    vehicle.current_lane = Some(LaneId::Normal(approach, 0));

    let mut engine = SimulationEngine::new(make_sim_config(map, 60.0), vec![blocker, vehicle]);
    engine.insert_vehicle_into_lane(LaneId::Normal(exit, 0), 0);
    engine.insert_vehicle_into_lane(LaneId::Normal(approach, 0), 1);
    engine.run();

    assert_eq!(engine.vehicles[1].current_lane, Some(LaneId::Normal(approach, 0)));
    assert!(engine.vehicles[1].position_on_lane < 500.0);
}
