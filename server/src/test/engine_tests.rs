use std::collections::HashMap;

use crate::simulation::engine::{lane_insert_sorted, Simulation, SimulationEngine};
use crate::simulation::vehicle::{LaneId, Vehicle, VehicleState};
use crate::test::{make_minimal_straight_map, make_sim_config, make_vehicle};
use petgraph::graph::{EdgeIndex, NodeIndex};

// ---- lane_insert_sorted unit tests ----

fn make_vehicle_at_position(id: u64, pos: f32) -> Vehicle {
    let mut v = make_vehicle(id, NodeIndex::new(0), NodeIndex::new(1));
    v.position_on_lane = pos;
    v
}

fn dummy_lane() -> LaneId {
    LaneId::Normal(EdgeIndex::new(0), 0)
}

#[test]
fn lane_insert_empty_lane() {
    let mut by_lane: HashMap<LaneId, Vec<usize>> = HashMap::new();
    let vehicles = vec![make_vehicle_at_position(0, 10.0)];
    lane_insert_sorted(&mut by_lane, &vehicles, dummy_lane(), 0);
    assert_eq!(by_lane[&dummy_lane()], vec![0]);
}

#[test]
fn lane_insert_maintains_ascending_order() {
    // Insert three vehicles at different positions; expect sorted ascending
    let vehicles = vec![
        make_vehicle_at_position(0, 10.0),
        make_vehicle_at_position(1, 5.0),
        make_vehicle_at_position(2, 20.0),
    ];
    let lane = dummy_lane();
    let mut by_lane: HashMap<LaneId, Vec<usize>> = HashMap::new();
    lane_insert_sorted(&mut by_lane, &vehicles, lane, 0); // pos=10
    lane_insert_sorted(&mut by_lane, &vehicles, lane, 1); // pos=5  → goes before 0
    lane_insert_sorted(&mut by_lane, &vehicles, lane, 2); // pos=20 → goes after 0
    assert_eq!(by_lane[&lane], vec![1, 0, 2]);
}

#[test]
fn lane_insert_front_of_queue() {
    // Vehicle with lowest position → index 0 (rearmost)
    let vehicles = vec![
        make_vehicle_at_position(0, 50.0),
        make_vehicle_at_position(1, 0.0),
    ];
    let lane = dummy_lane();
    let mut by_lane: HashMap<LaneId, Vec<usize>> = HashMap::new();
    lane_insert_sorted(&mut by_lane, &vehicles, lane, 0);
    lane_insert_sorted(&mut by_lane, &vehicles, lane, 1);
    assert_eq!(by_lane[&lane][0], 1); // rearmost first
}

#[test]
fn lane_insert_back_of_queue() {
    // Vehicle with highest position → last index
    let vehicles = vec![
        make_vehicle_at_position(0, 10.0),
        make_vehicle_at_position(1, 30.0),
        make_vehicle_at_position(2, 60.0),
        make_vehicle_at_position(3, 100.0),
    ];
    let lane = dummy_lane();
    let mut by_lane: HashMap<LaneId, Vec<usize>> = HashMap::new();
    for i in 0..4 {
        lane_insert_sorted(&mut by_lane, &vehicles, lane, i);
    }
    let list = &by_lane[&lane];
    assert_eq!(*list.last().unwrap(), 3);
}

#[test]
fn lane_insert_different_lanes_isolated() {
    let vehicles = vec![
        make_vehicle_at_position(0, 10.0),
        make_vehicle_at_position(1, 20.0),
    ];
    let lane_a = LaneId::Normal(EdgeIndex::new(0), 0);
    let lane_b = LaneId::Normal(EdgeIndex::new(1), 0);
    let mut by_lane: HashMap<LaneId, Vec<usize>> = HashMap::new();
    lane_insert_sorted(&mut by_lane, &vehicles, lane_a, 0);
    lane_insert_sorted(&mut by_lane, &vehicles, lane_b, 1);
    assert_eq!(by_lane[&lane_a].len(), 1);
    assert_eq!(by_lane[&lane_b].len(), 1);
    assert!(!by_lane[&lane_a].contains(&1));
    assert!(!by_lane[&lane_b].contains(&0));
}

// ---- Step-based integration tests ----

fn make_engine_with_one_vehicle() -> SimulationEngine {
    let map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let work = map.find_node(2).unwrap();
    let mut v = make_vehicle(0, hab, work);
    assert!(v.update_path(&map));
    let config = make_sim_config(map, 300.0);
    SimulationEngine::new(config, vec![v])
}

#[test]
fn step_vehicle_departs_onto_road() {
    let mut engine = make_engine_with_one_vehicle();
    assert_eq!(engine.vehicles[0].state, VehicleState::WaitingToDepart);
    engine.step();
    assert_eq!(engine.vehicles[0].state, VehicleState::OnRoad);
    assert!(engine.vehicles[0].current_lane.is_some());
}

#[test]
fn step_vehicle_advances_position() {
    let mut engine = make_engine_with_one_vehicle();
    // Run enough steps for the vehicle to depart and accelerate
    for _ in 0..30 {
        engine.step();
        engine.current_time += engine.config.time_step;
    }
    assert!(engine.vehicles[0].velocity > 0.0, "velocity should be positive");
}

#[test]
fn step_vehicle_velocity_bounded_by_speed_limit() {
    let mut engine = make_engine_with_one_vehicle();
    let speed_limit = 40.0f32;
    for _ in 0..400 {
        engine.step();
        engine.current_time += engine.config.time_step;
        if engine.vehicles[0].state == VehicleState::Arrived {
            break;
        }
        // Small tolerance for floating-point stepping
        assert!(
            engine.vehicles[0].velocity <= speed_limit + 1.0,
            "velocity {} exceeded speed limit {}",
            engine.vehicles[0].velocity,
            speed_limit
        );
    }
}

#[test]
fn step_velocity_never_negative() {
    let mut engine = make_engine_with_one_vehicle();
    for _ in 0..400 {
        engine.step();
        engine.current_time += engine.config.time_step;
        assert!(
            engine.vehicles[0].velocity >= 0.0,
            "velocity went negative: {}",
            engine.vehicles[0].velocity
        );
        if engine.vehicles[0].state == VehicleState::Arrived {
            break;
        }
    }
}

#[test]
fn step_two_vehicles_maintain_gap() {
    let map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let work = map.find_node(2).unwrap();

    let mut v0 = make_vehicle(0, hab, work);
    assert!(v0.update_path(&map));
    let mut v1 = make_vehicle(1, hab, work);
    assert!(v1.update_path(&map));

    let config = make_sim_config(map, 300.0);
    let mut engine = SimulationEngine::new(config, vec![v0, v1]);

    for _ in 0..500 {
        engine.step();
        engine.current_time += engine.config.time_step;

        // Find any lane with 2 vehicles and check gap
        for (_, indices) in &engine.vehicles_by_lane {
            if indices.len() < 2 {
                continue;
            }
            for pair in indices.windows(2) {
                let follower = &engine.vehicles[pair[0]];
                let leader = &engine.vehicles[pair[1]];
                let gap = leader.position_on_lane - leader.spec.length - follower.position_on_lane;
                assert!(
                    gap >= -0.5,
                    "vehicles overlap: gap={gap:.3}, follower.pos={}, leader.pos={}",
                    follower.position_on_lane,
                    leader.position_on_lane
                );
            }
        }

        if engine.vehicles.iter().all(|v| v.state == VehicleState::Arrived) {
            break;
        }
    }
}

#[test]
fn step_impatience_grows_when_waiting() {
    // Create a vehicle approaching a Stop link and verify waiting_time accumulates.
    // We manually set a Stop link on the first road by mutating the built map.
    let mut map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let jct = map.find_node(1).unwrap();
    let edge = map.graph.find_edge(hab, jct).unwrap();
    // Set all links on this lane to Stop so the vehicle must wait
    for lane in &mut map.graph[edge].lanes {
        for link in &mut lane.links {
            link.link_type = crate::map::road::LinkType::Stop;
        }
    }

    let work = map.find_node(2).unwrap();
    let mut v = make_vehicle(0, hab, work);
    assert!(v.update_path(&map));
    let config = make_sim_config(map, 300.0);
    let mut engine = SimulationEngine::new(config, vec![v]);

    // Run until vehicle has been on road for a while
    let mut max_waiting_time = 0.0f32;
    for _ in 0..2000 {
        engine.step();
        engine.current_time += engine.config.time_step;
        max_waiting_time = max_waiting_time.max(engine.vehicles[0].waiting_time);
        if engine.vehicles[0].state == VehicleState::Arrived {
            break;
        }
    }
    assert!(max_waiting_time > 0.0, "vehicle should have waited at the stop sign");
}

#[test]
fn step_impatience_resets_after_moving() {
    let map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let work = map.find_node(2).unwrap();
    let mut v = make_vehicle(0, hab, work);
    assert!(v.update_path(&map));
    let config = make_sim_config(map, 300.0);
    let mut engine = SimulationEngine::new(config, vec![v]);

    // Run the full simulation
    for _ in 0..6000 {
        engine.step();
        engine.current_time += engine.config.time_step;
        if engine.vehicles[0].state == VehicleState::Arrived {
            break;
        }
    }
    assert_eq!(engine.vehicles[0].state, VehicleState::Arrived);
    // After arriving, impatience should be zero (reset when moving freely)
    assert_eq!(engine.vehicles[0].impatience, 0.0);
}
