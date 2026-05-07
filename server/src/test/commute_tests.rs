use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use crate::api::runner::map_generator::create_random_commutes_with_rng;
use crate::api::runner::runner::SimulationInstance;
use crate::simulation::commute::CommutePlan;
use crate::simulation::commute::CommutePlanState;
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::simulation::vehicle::{TripRequest, Vehicle, VehicleState, VehicleType};
use crate::test::{make_minimal_straight_map, make_sim_config, make_standard_spec};

#[test]
fn commute_plan_new_is_deterministic() {
    let plan = CommutePlan::new(7, 11, 12, 1_234.5, 678.9);

    assert_eq!(plan.id, 7);
    assert_eq!(plan.outbound_vehicle_id, 11);
    assert_eq!(plan.return_vehicle_id, 12);
    assert_eq!(plan.outbound_departure_time_s, 1_234.5);
    assert_eq!(plan.return_waiting_time_s, 678.9);
    assert_eq!(plan.state, CommutePlanState::OutboundPending);
}

#[test]
fn commute_plan_random_is_seeded() {
    let mut rng_a = ChaCha20Rng::seed_from_u64(42);
    let mut rng_b = ChaCha20Rng::seed_from_u64(42);

    let plan_a = CommutePlan::random(0, 1, 2, 0.0, &mut rng_a);
    let plan_b = CommutePlan::random(0, 1, 2, 0.0, &mut rng_b);

    assert_eq!(plan_a.outbound_departure_time_s, plan_b.outbound_departure_time_s);
    assert_eq!(plan_a.return_waiting_time_s, plan_b.return_waiting_time_s);
    assert!((0.0..=43_200.0).contains(&plan_a.outbound_departure_time_s));
    assert!((0.0..=43_200.0).contains(&plan_a.return_waiting_time_s));
}

#[test]
fn commute_plan_random_respects_simulation_start_time() {
    let mut rng = ChaCha20Rng::seed_from_u64(99);
    let plan = CommutePlan::random(0, 1, 2, 39_600.0, &mut rng);

    assert!((39_600.0..=43_200.0).contains(&plan.outbound_departure_time_s));
}

#[test]
fn create_random_commutes_builds_paired_vehicles() {
    let map = make_minimal_straight_map();
    let mut rng = ChaCha20Rng::seed_from_u64(7);
    let generated = create_random_commutes_with_rng(&map, 2, &mut rng);

    assert_eq!(generated.vehicles.len(), 4);
    assert_eq!(generated.commute_plans.len(), 2);

    for (index, plan) in generated.commute_plans.iter().enumerate() {
        let outbound = &generated.vehicles[index * 2];
        let return_vehicle = &generated.vehicles[index * 2 + 1];

        assert_eq!(outbound.commute_plan_id, Some(plan.id));
        assert_eq!(return_vehicle.commute_plan_id, Some(plan.id));
        assert_eq!(outbound.trip.departure_time, plan.outbound_departure_time_s);
        assert_eq!(return_vehicle.trip.departure_time, f32::MAX);
        assert_eq!(plan.state, CommutePlanState::OutboundPending);
        assert_eq!(plan.outbound_vehicle_id, outbound.id);
        assert_eq!(plan.return_vehicle_id, return_vehicle.id);
    }
}

#[test]
fn create_random_commutes_is_seed_reproducible() {
    let map = make_minimal_straight_map();

    let mut rng_a = ChaCha20Rng::seed_from_u64(123);
    let mut rng_b = ChaCha20Rng::seed_from_u64(123);

    let generated_a = create_random_commutes_with_rng(&map, 3, &mut rng_a);
    let generated_b = create_random_commutes_with_rng(&map, 3, &mut rng_b);

    assert_eq!(generated_a.vehicles.len(), generated_b.vehicles.len());
    assert_eq!(generated_a.commute_plans.len(), generated_b.commute_plans.len());

    for (a, b) in generated_a.commute_plans.iter().zip(generated_b.commute_plans.iter()) {
        assert_eq!(a.outbound_departure_time_s, b.outbound_departure_time_s);
        assert_eq!(a.return_waiting_time_s, b.return_waiting_time_s);
    }

    for (a, b) in generated_a.vehicles.iter().zip(generated_b.vehicles.iter()) {
        assert_eq!(a.trip.origin, b.trip.origin);
        assert_eq!(a.trip.destination, b.trip.destination);
        assert_eq!(a.trip.departure_time, b.trip.departure_time);
        assert_eq!(a.motorization, b.motorization);
        assert_eq!(a.commute_plan_id, b.commute_plan_id);
    }
}

#[test]
fn commute_plan_runs_outbound_then_return() {
    let map = make_minimal_straight_map();
    let hab = map.find_node(0).unwrap();
    let work = map.find_node(2).unwrap();

    let spec = make_standard_spec();
    let mut outbound = Vehicle::new(
        0,
        spec,
        TripRequest {
            origin: hab,
            destination: work,
            departure_time: 0.0,
        },
        VehicleType::Essence,
    );
    outbound.commute_plan_id = Some(0);
    assert!(outbound.update_path(&map));

    let mut return_vehicle = Vehicle::new(
        1,
        spec,
        TripRequest {
            origin: work,
            destination: hab,
            departure_time: f32::MAX,
        },
        VehicleType::Essence,
    );
    return_vehicle.commute_plan_id = Some(0);
    assert!(return_vehicle.update_path(&map));

    let config = make_sim_config(map, 500.0);
    let mut engine = SimulationEngine::new_with_commutes(
        config,
        vec![outbound, return_vehicle],
        vec![CommutePlan::new(0, 0, 1, 0.0, 0.0)],
    );

    engine.run();

    assert!(engine.vehicles.iter().all(|vehicle| vehicle.state == VehicleState::Arrived));
    let plan = engine.commute_plans.get(&0).expect("commute plan should exist");
    assert_eq!(plan.state, CommutePlanState::Completed);
}

#[tokio::test]
async fn simulation_instance_uses_front_vehicle_count_as_plan_count() {
    let mut map = make_minimal_straight_map();
    map.settings.vehicle_count = 3;
    map.settings.simulation_duration = 120.0;

    let instance = SimulationInstance::new(map);
    let eng = instance.initial_engine.lock().await;

    assert_eq!(eng.commute_plans.len(), 3);
    assert_eq!(eng.vehicles.len(), 6);
    assert!(eng.vehicles.iter().all(|vehicle| vehicle.commute_plan_id.is_some()));
}