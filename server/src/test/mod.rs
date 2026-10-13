mod kinematics_tests;
mod vehicle_tests;
mod intersection_tests;
mod editor_tests;
mod engine_tests;
mod commute_tests;
mod simulation_tests;
mod score_progress_tests;
mod lane_change_tests;
mod scoring_tests;
mod right_of_way_tests;
mod regression_tests;

use petgraph::graph::NodeIndex;

use crate::map::intersection::{build_intersections, IntersectionKind};
use crate::map::model::Map;
use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{TripRequest, Vehicle, VehicleKind, VehicleSpec, VehicleType};

pub(crate) fn make_standard_spec() -> VehicleSpec {
    VehicleSpec::new(VehicleKind::Car, 40.0, 4.0, 3.0, 1.0, 10.0)
}

pub(crate) fn make_vehicle(id: u64, origin: NodeIndex, dest: NodeIndex) -> Vehicle {
    Vehicle::new(
        id,
        make_standard_spec(),
        TripRequest {
            origin,
            destination: dest,
            departure_time: 0.0,
        },
        VehicleType::Essence,
    )
}

/// Creates a simple 3-node straight map: hab(0,0) → jct(500,0) → work(1000,0).
/// Node IDs are 0 (hab), 1 (jct), 2 (work). Two-way roads at 40 m/s.
/// `build_intersections` is called before returning.
pub(crate) fn make_minimal_straight_map() -> Map {
    let mut map = Map::new();
    let _hab = map.add_intersection(IntersectionKind::Habitation, 0.0, 0.0);
    let _jct = map.add_intersection(IntersectionKind::Intersection, 500.0, 0.0);
    let _work = map.add_intersection(IntersectionKind::Workplace, 1000.0, 0.0);
    map.add_two_way_road(0, 1, 1, 40.0, 500.0);
    map.add_two_way_road(1, 2, 1, 40.0, 500.0);
    build_intersections(&mut map);
    map
}

pub(crate) fn make_sim_config(map: Map, end_time: f32) -> SimulationConfig {
    use crate::simulation::config::ScoreWeights;
    SimulationConfig {
        start_time: 0.0,
        end_time,
        time_step: 0.1,
        minimum_gap: 2.0,
        score_weights: ScoreWeights::from_settings(&map.settings),
        map,
    }
}

/// Id of the link on lane 0 of road `from -> via` that continues to road `via -> to`.
pub(crate) fn link_between(map: &Map, from: u32, via: u32, to: u32) -> u32 {
    let (from, via, to) = (
        map.find_node(from).unwrap(),
        map.find_node(via).unwrap(),
        map.find_node(to).unwrap(),
    );
    let in_edge = map.graph.find_edge(from, via).unwrap();
    let out_road = map.graph[map.graph.find_edge(via, to).unwrap()].id;
    map.graph[in_edge].lanes[0]
        .links
        .iter()
        .find(|link| link.destination_road_id == out_road)
        .expect("link should exist")
        .id
}

/// A vehicle going `from -> ... -> to` (node ids), departing at `departure_time`.
pub(crate) fn make_trip(map: &Map, id: u64, from: u32, to: u32, departure_time: f32) -> Vehicle {
    let mut vehicle = make_vehicle(id, map.find_node(from).unwrap(), map.find_node(to).unwrap());
    vehicle.trip.departure_time = departure_time;
    assert!(vehicle.update_path(map), "route should exist");
    vehicle
}

/// Steps until every vehicle arrived or `max_time` is reached, checking after every step
/// that no two vehicles are inside a junction on conflicting paths.
pub(crate) fn run_checked(
    engine: &mut crate::simulation::engine::SimulationEngine,
    max_time: f32,
) -> bool {
    use crate::simulation::engine::Simulation;
    use crate::simulation::vehicle::{LaneId, VehicleState};
    use std::collections::{HashMap, HashSet};

    let mut foes: HashMap<LaneId, HashSet<LaneId>> = HashMap::new();
    for node in engine.config.map.graph.node_indices() {
        let junction_id = engine.config.map.graph[node].id;
        for edge in engine.config.map.graph.edges_directed(node, petgraph::Direction::Incoming) {
            for link in edge.weight().lanes.iter().flat_map(|lane| &lane.links) {
                foes.insert(
                    LaneId::Internal(junction_id, link.via_internal_lane_id),
                    link.foe_internal_lane_ids
                        .iter()
                        .map(|&il| LaneId::Internal(junction_id, il))
                        .collect(),
                );
            }
        }
    }

    while engine.current_time < max_time {
        engine.step();
        engine.current_time += engine.config.time_step;

        for (lane, list) in &engine.vehicles_by_lane {
            if let Some(conflicting) = foes.get(lane) {
                for other in conflicting {
                    assert!(
                        !engine.vehicles_by_lane.contains_key(other),
                        "t={:.1}: vehicles {:?} on {lane:?} and {:?} on {other:?} cross each other",
                        engine.current_time,
                        list.iter().map(|&i| engine.vehicles[i].id).collect::<Vec<_>>(),
                        engine.vehicles_by_lane[other].iter().map(|&i| engine.vehicles[i].id).collect::<Vec<_>>(),
                    );
                }
            }
            for pair in list.windows(2) {
                let (follower, leader) = (&engine.vehicles[pair[0]], &engine.vehicles[pair[1]]);
                let gap = leader.position_on_lane - leader.spec.length - follower.position_on_lane;
                assert!(gap > -0.5, "t={:.1}: vehicles {} and {} overlap by {gap}", engine.current_time, follower.id, leader.id);
            }
        }

        if engine.vehicles.iter().all(|v| v.state == VehicleState::Arrived) {
            return true;
        }
    }
    false
}
