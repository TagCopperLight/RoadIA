//! One test per engine bug found while reviewing it.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::api::runner::map_generator::{create_intersection_test_map, create_random_commutes_with_rng};
use crate::map::intersection::{build_intersections, IntersectionKind};
use crate::map::model::{Map, SavedBusLine};
use crate::simulation::commute::{CommutePlan, CommutePlanState};
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::simulation::vehicle::{LaneId, VehicleState};
use crate::test::{make_minimal_straight_map, make_sim_config, make_trip, run_checked};

/// An n x n grid of two-way roads with priority to the right everywhere.
fn grid_map(n: u32) -> Map {
    let mut map = Map::new();
    for y in 0..n {
        for x in 0..n {
            map.add_intersection(IntersectionKind::Intersection, x as f32 * 150.0, y as f32 * 150.0);
        }
    }
    for y in 0..n {
        for x in 0..n {
            let id = y * n + x;
            if x + 1 < n {
                map.add_two_way_road(id, id + 1, 1, 14.0, 140.0);
            }
            if y + 1 < n {
                map.add_two_way_road(id, id + n, 1, 14.0, 140.0);
            }
        }
    }
    build_intersections(&mut map);
    map
}

#[test]
fn busy_grid_never_gridlocks_or_collides() {
    // The old engine left a third of these vehicles stuck forever at 4-way junctions
    // where every approach yielded to the one on its right.
    let map = grid_map(5);
    let border: Vec<u32> = (0..25).filter(|id| id % 5 == 0 || id % 5 == 4 || id / 5 == 0 || id / 5 == 4).collect();
    let vehicles = (0..400)
        .map(|i| {
            let from = border[i % border.len()];
            let to = border[(i * 7 + 5) % border.len()];
            let to = if to == from { border[(i + 1) % border.len()] } else { to };
            make_trip(&map, i as u64, from, to, (i / 16) as f32)
        })
        .collect();
    let mut engine = SimulationEngine::new(make_sim_config(map, 1200.0), vehicles);
    engine.set_seed(1);
    assert!(run_checked(&mut engine, 1200.0), "all vehicles should arrive");
}

#[test]
fn simulation_is_reproducible_with_a_seed() {
    let run = || {
        let map = grid_map(4);
        let vehicles = (0..40)
            .map(|i| make_trip(&map, i, (i % 4) as u32, 15 - (i % 4) as u32, i as f32 * 0.5))
            .collect();
        let mut engine = SimulationEngine::new(make_sim_config(map, 120.0), vehicles);
        engine.set_seed(42);
        engine.run();
        engine.vehicles.iter().map(|v| (v.position_on_lane, v.current_lane)).collect::<Vec<_>>()
    };
    assert_eq!(run(), run());
}

#[test]
fn arrived_vehicles_leave_no_registration_behind() {
    // Registrations of arrived vehicles used to stay on the links forever.
    let map = create_intersection_test_map();
    let vehicles = vec![make_trip(&map, 0, 1, 2, 0.0), make_trip(&map, 1, 3, 4, 0.0)];
    let mut engine = SimulationEngine::new(make_sim_config(map, 300.0), vehicles);
    engine.run();
    for vidx in 0..2 {
        assert_eq!(engine.vehicles[vidx].state, VehicleState::Arrived);
        assert_eq!(engine.approach_count(vidx), 0);
    }
}

#[test]
fn lane_change_does_not_land_on_a_vehicle_at_the_same_position() {
    // Two vehicles departing together on a two-lane road start side by side; the
    // keep-right logic used to merge one onto the other.
    let mut map = Map::new();
    map.add_intersection(IntersectionKind::Habitation, 0.0, 0.0);
    map.add_intersection(IntersectionKind::Workplace, 1000.0, 0.0);
    map.add_road(0, 1, 2, 30.0, 1000.0);
    build_intersections(&mut map);

    let vehicles = vec![make_trip(&map, 0, 0, 1, 0.0), make_trip(&map, 1, 0, 1, 0.0)];
    let mut engine = SimulationEngine::new(make_sim_config(map, 120.0), vehicles);
    assert!(run_checked(&mut engine, 120.0));
}

#[test]
fn commute_return_never_hijacks_another_vehicle() {
    // A one-way commute used to point its return trip at the next commute's outbound
    // vehicle, which then got teleported back to its origin while driving.
    let map = make_minimal_straight_map();
    let short = make_trip(&map, 0, 0, 1, 0.0);
    let long = make_trip(&map, 1, 0, 2, 0.0);
    let mut short = short;
    short.commute_plan_id = Some(0);
    let mut long = long;
    long.commute_plan_id = Some(1);
    long.trip.departure_time = 2.0;

    let plans = vec![
        // Corrupted plan: its "return vehicle" belongs to the other commute.
        CommutePlan::new(0, 0, Some(1), 0.0, 0.0),
        CommutePlan::new(1, 1, None, 2.0, 0.0),
    ];
    let mut engine = SimulationEngine::new_with_commutes(make_sim_config(map, 300.0), vec![short, long], plans);
    assert!(run_checked(&mut engine, 300.0));
    assert_eq!(engine.commute_plans[&0].state, CommutePlanState::Failed);
    assert_eq!(engine.vehicles[1].trip.departure_time, 2.0);
}

#[test]
fn one_way_commutes_have_no_return_vehicle() {
    let mut map = Map::new();
    map.add_intersection(IntersectionKind::Habitation, 0.0, 0.0);
    map.add_intersection(IntersectionKind::Intersection, 500.0, 0.0);
    map.add_intersection(IntersectionKind::Workplace, 1000.0, 0.0);
    map.add_road(0, 1, 1, 30.0, 500.0);
    map.add_road(1, 2, 1, 30.0, 500.0);
    build_intersections(&mut map);

    let mut rng = ChaCha8Rng::seed_from_u64(3);
    let generated = create_random_commutes_with_rng(&map, 5, &mut rng);
    assert_eq!(generated.vehicles.len(), 5);
    for plan in &generated.commute_plans {
        assert_eq!(plan.return_vehicle_id, None);
    }
}

#[test]
fn bus_turns_around_at_a_dead_end_stop() {
    // The route 0 -> 1 -> 2 -> 1 -> 0 needs a U-turn at the dead end 2; vehicles used
    // to get stuck there forever.
    let mut map = make_minimal_straight_map();
    map.bus_lines.push(SavedBusLine { id: 0, name: "line".into(), stop_node_ids: vec![0, 2] });
    let line = map.bus_lines[0].clone();
    let mut engine = SimulationEngine::new(make_sim_config(map, 400.0), Vec::new());
    assert!(engine.add_bus_from_saved(&line));
    engine.run();
    // Both legs are 1000 m long: a stuck bus would never get past 1000 m.
    assert!(engine.vehicles[0].distance_traveled > 2500.0, "bus should loop: {}", engine.vehicles[0].distance_traveled);
}

#[test]
fn vehicles_inside_a_junction_keep_their_plan() {
    // Plans used to be dropped while crossing a junction, hiding the vehicle from the
    // foes of the next junction.
    let mut map = Map::new();
    for x in 0..4 {
        map.add_intersection(IntersectionKind::Intersection, x as f32 * 50.0, 0.0);
    }
    for id in 0..3 {
        map.add_two_way_road(id, id + 1, 1, 14.0, 40.0);
    }
    build_intersections(&mut map);

    let vehicle = make_trip(&map, 0, 0, 3, 0.0);
    let mut engine = SimulationEngine::new(make_sim_config(map, 100.0), vec![vehicle]);
    while !matches!(engine.vehicles[0].current_lane, Some(LaneId::Internal(..))) {
        engine.step();
        engine.current_time += engine.config.time_step;
        assert!(engine.current_time < 100.0, "vehicle should reach the first junction");
    }
    engine.step();
    assert!(matches!(engine.vehicles[0].current_lane, Some(LaneId::Internal(..))));
    assert!(!engine.vehicles[0].drive_plan.is_empty());
    assert!(engine.approach_count(0) > 0);
}
