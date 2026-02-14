use crate::map::intersection::{Intersection, IntersectionKind};
use crate::map::road::Road;
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::simulation::vehicle::{
    fastest_path, TripRequest, Vehicle, VehicleKind, VehicleSpec, VehicleState,
};

fn all_arrived(sim: &SimulationEngine) -> bool {
    sim.vehicles.iter().all(|v| v.state == VehicleState::Arrived)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{map::model::Map, simulation::config::SimulationConfig};

    #[test]
    fn test_simulation_engine_creation_and_step() {
        let map = Map::default();
        let vehicles = vec![];
        let config = SimulationConfig {
            start_time: 0.0,
            end_time: 10.0,
            time_step: 1.0,
            minimum_gap: 1.0,
            map,
        };
        let mut sim = SimulationEngine::new(config, vehicles);

        assert_eq!(sim.current_time, 0.0);
        assert_eq!(sim.config.end_time, 10.0);

        sim.step();

        sim.run();
        assert!(sim.current_time == sim.config.end_time);
    }

    #[test]
    fn test_simulation_engine_vehicle_movement() {
        let mut map = Map::new();

        let h1 = map.add_intersection(Intersection::new(
            0,
            IntersectionKind::Habitation,
            "h1".into(),
            0.0,
            0.0,
        ));

        let h2 = map.add_intersection(Intersection::new(
            1,
            IntersectionKind::Habitation,
            "h2".into(),
            0.0,
            100.0,
        ));

        let i1 = map.add_intersection(Intersection::new(
            2,
            IntersectionKind::Intersection,
            "i1".into(),
            50.0,
            50.0,
        ));

        let w1 = map.add_intersection(Intersection::new(
            3,
            IntersectionKind::Workplace,
            "w1".into(),
            100.0,
            50.0,
        ));

        map.add_two_way_road(h1, i1, Road::new(0, 1, 50.0, 100.0, false, false));
        map.add_two_way_road(h2, i1, Road::new(1, 1, 50.0, 100.0, false, false));
        map.add_two_way_road(i1, w1, Road::new(2, 1, 100.0, 100.0, false, false));

        let config = SimulationConfig {
            start_time: 0.0,
            end_time: 10.0,
            time_step: 0.1,
            minimum_gap: 1.0,
            map: map.clone(),
        };

        let mut vehicles: Vec<Vehicle> = Vec::new();

        let spec = VehicleSpec {
            kind: VehicleKind::Car,
            max_speed: 100.0,
            max_acceleration: 20.0,
            comfortable_deceleration: 1.67,
            reaction_time: 1.0,
            length: 5.0,
        };

        let path = fastest_path(&map, h1, w1);

        vehicles.push(Vehicle {
            id: 0,
            spec,
            trip: TripRequest {
                origin: h1,
                destination: w1,
                departure_time: 0,
                return_time: None,
            },
            state: VehicleState::WaitingToDepart,
            path,
            path_index: 0,
            position_on_road: 0.0,
            previous_position: 0.0,
            velocity: 0.0,
            previous_velocity: 0.0,
        });

        let mut sim = SimulationEngine::new(config, vehicles);
        sim.run();
        assert!(all_arrived(&sim));
    }

    #[test]
    fn test_simulation_engine_multiple_vehicles() {
        let mut map = Map::new();

        let h1 = map.add_intersection(Intersection::new(
            0,
            IntersectionKind::Habitation,
            "h1".into(),
            0.0,
            0.0,
        ));

        let h2 = map.add_intersection(Intersection::new(
            1,
            IntersectionKind::Habitation,
            "h2".into(),
            0.0,
            100.0,
        ));

        let i1 = map.add_intersection(Intersection::new(
            2,
            IntersectionKind::Intersection,
            "i1".into(),
            50.0,
            50.0,
        ));

        let w1 = map.add_intersection(Intersection::new(
            3,
            IntersectionKind::Workplace,
            "w1".into(),
            100.0,
            50.0,
        ));

        map.add_two_way_road(h1, i1, Road::new(0, 1, 50.0, 100.0, false, false));
        map.add_two_way_road(h2, i1, Road::new(1, 1, 50.0, 100.0, false, false));
        map.add_two_way_road(i1, w1, Road::new(2, 1, 100.0, 100.0, false, false));

        let config = SimulationConfig {
            start_time: 0.0,
            end_time: 10.0,
            time_step: 0.1,
            minimum_gap: 1.0,
            map: map.clone(),
        };

        let mut vehicles: Vec<Vehicle> = Vec::new();

        let spec = VehicleSpec {
            kind: VehicleKind::Car,
            max_speed: 100.0,
            max_acceleration: 20.0,
            comfortable_deceleration: 1.67,
            reaction_time: 1.0,
            length: 5.0,
        };

        let path0 = fastest_path(&map, h1, w1);

        vehicles.push(Vehicle {
            id: 0,
            spec,
            trip: TripRequest {
                origin: h1,
                destination: w1,
                departure_time: 0,
                return_time: None,
            },
            state: VehicleState::WaitingToDepart,
            path: path0,
            path_index: 0,
            position_on_road: 0.0,
            previous_position: 0.0,
            velocity: 0.0,
            previous_velocity: 0.0,
        });

        let path1 = fastest_path(&map, h2, w1);

        vehicles.push(Vehicle {
            id: 1,
            spec,
            trip: TripRequest {
                origin: h2,
                destination: w1,
                departure_time: 0,
                return_time: None,
            },
            state: VehicleState::WaitingToDepart,
            path: path1,
            path_index: 0,
            position_on_road: 0.0,
            previous_position: 0.0,
            velocity: 0.0,
            previous_velocity: 0.0,
        });

        let mut sim = SimulationEngine::new(config, vehicles);
        sim.run();
        assert!(all_arrived(&sim));
    }
}
