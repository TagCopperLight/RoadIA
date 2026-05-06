use crate::api::runner::map_generator::create_random_commutes;
use crate::api::runner::runner::SimulationInstance;
use crate::simulation::commute::CommutePlanState;
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::simulation::vehicle::VehicleState;
use crate::test::{make_minimal_straight_map, make_sim_config};

#[test]
fn create_random_commutes_builds_paired_vehicles() {
    let map = make_minimal_straight_map();
    let generated = create_random_commutes(&map, 2);

    assert_eq!(generated.vehicles.len(), 4);
    assert_eq!(generated.commute_plans.len(), 2);

    for (index, plan) in generated.commute_plans.iter().enumerate() {
        let outbound = &generated.vehicles[index * 2];
        let return_vehicle = &generated.vehicles[index * 2 + 1];

        assert_eq!(outbound.commute_plan_id, Some(plan.id));
        assert_eq!(return_vehicle.commute_plan_id, Some(plan.id));
        assert_eq!(outbound.trip.departure_time, 0.0);
        assert_eq!(return_vehicle.trip.departure_time, f32::MAX);
        assert_eq!(plan.state, CommutePlanState::OutboundPending);
        assert_eq!(plan.outbound_vehicle_id, outbound.id);
        assert_eq!(plan.return_vehicle_id, return_vehicle.id);
    }
}

#[test]
fn commute_plan_runs_outbound_then_return() {
    let map = make_minimal_straight_map();
    let generated = create_random_commutes(&map, 1);
    let config = make_sim_config(map, 500.0);
    let mut engine = SimulationEngine::new_with_commutes(
        config,
        generated.vehicles,
        generated.commute_plans,
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