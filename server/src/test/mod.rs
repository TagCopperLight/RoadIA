mod kinematics_tests;
mod vehicle_tests;
mod intersection_tests;
mod editor_tests;
mod engine_tests;
mod simulation_tests;

use petgraph::graph::NodeIndex;

use crate::map::intersection::{build_intersections, ApproachData, IntersectionKind};
use crate::map::model::Map;
use crate::map::road::{FoeLink, Link, LinkType};
use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{TripRequest, Vehicle, VehicleKind, VehicleSpec};

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
    SimulationConfig {
        start_time: 0.0,
        end_time,
        time_step: 0.05,
        minimum_gap: 2.0,
        map,
    }
}

/// Builds a minimal Link with Priority type, no foes, dummy coordinates.
pub(crate) fn make_default_link(id: u32) -> Link {
    Link {
        id,
        lane_origin_id: 0,
        lane_destination_id: 0,
        via_internal_lane_id: 0,
        destination_road_id: 0,
        link_type: LinkType::Priority,
        entry: (0.0, 0.0),
        junction_center: (10.0, 0.0),
        foe_links: vec![],
        foe_internal_lane_ids: vec![],
    }
}

pub(crate) fn make_approach_data(arrival: f32, leave: f32) -> ApproachData {
    ApproachData {
        arrival_time: arrival,
        leave_time: leave,
        arrival_speed: 10.0,
        leave_speed: 14.0,
        will_pass: true,
    }
}

/// Creates a dummy FoeLink with the given type and entry point.
pub(crate) fn make_foe_link(id: u32, link_type: LinkType, entry: (f32, f32)) -> FoeLink {
    FoeLink { id, link_type, entry }
}
