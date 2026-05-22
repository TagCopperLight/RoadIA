use petgraph::graph::NodeIndex;

use crate::map::intersection::{build_intersections, IntersectionKind};
use crate::map::model::Map;
use crate::scoring::{
    emission_params, get_minimal_co2_by_road, get_minimal_time_travel_by_road,
    steiner_lower_bound, update_co2_emissions,
};
use crate::simulation::vehicle::{TripRequest, Vehicle, VehicleKind, VehicleType};
use crate::test::make_standard_spec;

#[test]
fn test_emission_params() {
    // Essence vs Diesel vs Electrique vs Hybride for a Car
    let (essence_mech, essence_idle) = emission_params(VehicleType::Essence, VehicleKind::Car);
    let (diesel_mech, diesel_idle) = emission_params(VehicleType::Diesel, VehicleKind::Car);
    let (elec_mech, elec_idle) = emission_params(VehicleType::Electrique, VehicleKind::Car);
    let (hybrid_mech, hybrid_idle) = emission_params(VehicleType::Hybride, VehicleKind::Car);

    // Mechanical emissions (g/J) assertions:
    // Diesel is more thermally efficient (42%) than petrol (35%), hence lower CO2/mechanical work.
    assert!(diesel_mech < essence_mech);
    // Electric emissions are from the clean grid, extremely low.
    assert!(elec_mech < diesel_mech);
    // Hybrid is halfway between petrol (38% efficiency model) and electric.
    assert!(hybrid_mech > elec_mech && hybrid_mech < essence_mech);

    // Idle emissions (g/s) assertions:
    assert!(elec_idle < essence_idle);
    assert!(hybrid_idle < essence_idle);
}

#[test]
fn test_minimal_time_travel_by_road() {
    let mut map = Map::new();
    let n1 = map.add_intersection(IntersectionKind::Habitation, 0.0, 0.0);
    let n2 = map.add_intersection(IntersectionKind::Workplace, 200.0, 0.0);
    let road_id = map.add_road(n1, n2, 1, 30.0, 200.0);
    let road_edge_idx = map.find_edge(road_id).unwrap();

    // Test when max speed is reached (long road)
    // acceleration = 4.0 m/s^2, max_speed = 20.0 m/s.
    // Accel phase: length = 0.5 * 20^2 / 4 = 50m. Accel time = 20 / 4 = 5s.
    // Cruise phase: length = 200 - 50 = 150m. Cruise time = 150 / 20 = 7.5s.
    // Total time = 5.0 + 7.5 = 12.5s.
    let time = get_minimal_time_travel_by_road(&map, road_edge_idx, 4.0, 20.0);
    assert!((time - 12.5).abs() < 1e-4);

    // Test when speed limit restricts vehicle max speed
    // speed_limit = 30.0m/s. Vehicle max speed = 10.0m/s.
    // Accel phase: length = 0.5 * 10^2 / 4 = 12.5m. Accel time = 10 / 4 = 2.5s.
    // Cruise phase: length = 200 - 12.5 = 187.5m. Cruise time = 187.5 / 10 = 18.75s.
    // Total time = 2.5 + 18.75 = 21.25s.
    let time_slow = get_minimal_time_travel_by_road(&map, road_edge_idx, 4.0, 10.0);
    assert!((time_slow - 21.25).abs() < 1e-4);

    // Test when road is too short to reach max speed (only acceleration)
    // road length = 50m. Accel phase needs 0.5 * 30^2 / 4 = 112.5m.
    // Uses: sqrt(2 * length / acceleration) = sqrt(2 * 50 / 4) = 5.0s.
    let n3 = map.add_intersection(IntersectionKind::Intersection, 50.0, 0.0);
    let short_road_id = map.add_road(n1, n3, 1, 30.0, 50.0);
    let short_road_edge_idx = map.find_edge(short_road_id).unwrap();
    let time_short = get_minimal_time_travel_by_road(&map, short_road_edge_idx, 4.0, 30.0);
    assert!((time_short - 5.0).abs() < 1e-4);
}

#[test]
fn test_minimal_co2_by_road() {
    let mut map = Map::new();
    let n1 = map.add_intersection(IntersectionKind::Habitation, 0.0, 0.0);
    let n2 = map.add_intersection(IntersectionKind::Workplace, 1000.0, 0.0);
    let road_id = map.add_road(n1, n2, 1, 50.0, 1000.0);
    let road_edge_idx = map.find_edge(road_id).unwrap();

    // Let's ensure minimal CO2 function runs and gives a positive cost
    let co2_essence = get_minimal_co2_by_road(&map, road_edge_idx, VehicleType::Essence, VehicleKind::Car);
    let co2_elec = get_minimal_co2_by_road(&map, road_edge_idx, VehicleType::Electrique, VehicleKind::Car);

    assert!(co2_essence > 0.0);
    assert!(co2_elec > 0.0);
    // Electric should emit significantly less CO2 than petrol
    assert!(co2_elec < co2_essence);
}

#[test]
fn test_update_co2_emissions() {
    let mut vehicle = Vehicle::new(
        1,
        make_standard_spec(),
        TripRequest {
            origin: NodeIndex::new(0),
            destination: NodeIndex::new(1),
            departure_time: 0.0,
        },
        VehicleType::Essence,
    );

    // Initially 0 emissions
    assert_eq!(vehicle.emitted_co2, 0.0);

    // Update with vehicle stationary (standstill) -> emits idle CO2
    // Idle CO2/s for petrol car: k.idle_combustion_w (2500W) * 3.16 / (0.35 * 43,200,000) = ~0.000522 kg/s
    update_co2_emissions(&mut vehicle, 1.0);
    assert!(vehicle.emitted_co2 > 0.0);
    let idle_one_sec = vehicle.emitted_co2;

    // Update with vehicle moving and accelerating
    vehicle.previous_velocity = 0.0;
    vehicle.velocity = 10.0; // Accel = 10.0 m/s^2, average velocity = 10m/s
    update_co2_emissions(&mut vehicle, 1.0);
    
    // Emissions during active acceleration should be far greater than idle emissions
    let active_emissions = vehicle.emitted_co2 - idle_one_sec;
    assert!(active_emissions > idle_one_sec);
}

#[test]
fn test_steiner_lower_bound_calculation() {
    let mut map = Map::new();
    
    // Empty map
    assert_eq!(steiner_lower_bound(&map), 0.0);

    // Minimal straight line of points
    let _h1 = map.add_intersection(IntersectionKind::Habitation, 0.0, 0.0);
    let _w1 = map.add_intersection(IntersectionKind::Workplace, 100.0, 0.0);
    build_intersections(&mut map);

    let bound = steiner_lower_bound(&map);
    // mst_length should be 100.0. Steiner bound = (3.0_f64.sqrt() / 2.0) * 100.0 = ~86.6025
    assert!((bound - 86.6025).abs() < 1e-2);
}
