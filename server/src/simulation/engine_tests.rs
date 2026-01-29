use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::map::intersection::{Intersection, IntersectionKind};
use crate::map::road::{Road};
use crate::simulation::vehicle::{Vehicle, VehicleKind, VehicleSpec, VehicleState, TripRequest, fastest_path};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{map::model::Map, simulation::config::SimulationConfig};
    // use crate::simulation::config::SimulationConfig; // Engine creates config now
    // use crate::simulation::vehicle::Vehicle;

    #[test]
    fn test_simulation_engine_creation_and_step() {
        let map = Map::default();
        let vehicles = vec![];
        let config = SimulationConfig {
            start_time_s: 0.0,
            end_time_s: 10.0,
            time_step_s: 1.0,
            acceleration_exponent: 4.0,
            minimum_gap: 1.0,
            map,
        };
        let mut sim = SimulationEngine::new(config, vehicles);

        assert_eq!(sim.current_time, 0.0);
        assert_eq!(sim.config.end_time_s, 10.0);

        sim.step();
        // step() does NOT increment current_time, run() does.
        // check if step runs without panic

        sim.run();
        assert!(sim.current_time >= 10.0);
    }

    #[test]
    fn test_simulation_engine_vehicle_movement(){
        let mut map = Map::new();

        let h1 = map.add_intersection(
            Intersection {
                id:0,
                kind: IntersectionKind::Habitation,
                name:"h1".into(),
                x:0.0,
                y:0.0
            }
        );

        let h2 = map.add_intersection(
            Intersection {
                id:1,
                kind: IntersectionKind::Habitation,
                name: "h2".into(),
                x: 0.0,
                y: 100.0
            }
        );

        let i1 = map.add_intersection(
            Intersection {
                id:2,
                kind: IntersectionKind::Intersection,
                name: "i1".into(),
                x: 50.0,
                y: 50.0
            }
        );

        let w1 = map.add_intersection(
            Intersection {
                id: 3,
                kind: IntersectionKind::Workplace,
                name: "w1".into(),
                x: 100.0,
                y: 50.0
            }
        );

        map.add_two_way_road(h1, i1, Road::new(0, 1, 50, 100.0, false, false));
        map.add_two_way_road(h2, i1, Road::new(1, 1, 50, 100.0, false, false));
        map.add_two_way_road(i1, w1, Road::new(2, 1, 100, 100.0, false, false));

        let config = SimulationConfig {
            start_time_s : 0.0,
            end_time_s : 10.0,
            time_step_s : 0.1,
            acceleration_exponent : 4.0,
            minimum_gap : 1.0,
            map: map.clone()
        };

        let mut vehicles : Vec<Vehicle> = Vec::new();

        let spec = VehicleSpec {
            kind: VehicleKind::Car,
            max_speed_ms: 100.0,
            max_acceleration_ms2: 20.0,
            comfortable_deceleration: 1.67,
            reaction_time: 1.0,
            length_m: 5.0,
            fuel_consumption_l_per_100km: 2.0,
            co2_g_per_km: 1.0,
        };

        let path = fastest_path(&map, h1, w1);

        vehicles.push(
            Vehicle {
                id:0,
                spec,
                trip: TripRequest {
                    origin_id: 0,
                    destination_id: 3,
                    departure_time_s: 0,
                    return_time_s: None,
                },
                state : VehicleState::WaitingToDepart,
                current_node: h1,
                next_node: Some(i1),
                path,
                path_index: 0,
                position_on_edge_m: 0.0,
                previous_position: 0.0,
                velocity: 0.0,
                previous_velocity: 0.0,
                distance_travelled_m: 0.0,
                fuel_used_l: 0.0,
                co2_emitted_g: 0.0,
                intersection_wait_start_time_s: None
            }
        );

        let mut sim = SimulationEngine::new(config, vehicles);
        sim.run();
    }

    #[test]
    fn test_simulation_engine_multiple_vehicles(){
        let mut map = Map::new();

        let h1 = map.add_intersection(
            Intersection {
                id:0,
                kind: IntersectionKind::Habitation,
                name:"h1".into(),
                x:0.0,
                y:0.0
            }
        );

        let h2 = map.add_intersection(
            Intersection {
                id:1,
                kind: IntersectionKind::Habitation,
                name: "h2".into(),
                x: 0.0,
                y: 100.0
            }
        );

        let i1 = map.add_intersection(
            Intersection {
                id:2,
                kind: IntersectionKind::Intersection,
                name: "i1".into(),
                x: 50.0,
                y: 50.0
            }
        );

        let w1 = map.add_intersection(
            Intersection {
                id: 3,
                kind: IntersectionKind::Workplace,
                name: "w1".into(),
                x: 100.0,
                y: 50.0
            }
        );

        map.add_two_way_road(h1, i1, Road::new(0, 1, 50, 100.0, false, false));
        map.add_two_way_road(h2, i1, Road::new(1, 1, 50, 100.0, false, false));
        map.add_two_way_road(i1, w1, Road::new(2, 1, 100, 100.0, false, false));

        let config = SimulationConfig {
            start_time_s : 0.0,
            end_time_s : 10.0,
            time_step_s : 0.1,
            acceleration_exponent : 4.0,
            minimum_gap : 1.0,
            map: map.clone()
        };

        let mut vehicles : Vec<Vehicle> = Vec::new();

        let spec = VehicleSpec {
            kind: VehicleKind::Car,
            max_speed_ms: 100.0,
            max_acceleration_ms2: 20.0,
            comfortable_deceleration: 1.67,
            reaction_time: 1.0,
            length_m: 5.0,
            fuel_consumption_l_per_100km: 2.0,
            co2_g_per_km: 1.0,
        };

        let path0 = fastest_path(&map, h1, w1);

        vehicles.push(
            Vehicle {
                id:0,
                spec,
                trip: TripRequest {
                    origin_id: 0,
                    destination_id: 3,
                    departure_time_s: 0,
                    return_time_s: None,
                },
                state : VehicleState::WaitingToDepart,
                current_node: h1,
                next_node: Some(i1),
                path : path0,
                path_index: 0,
                position_on_edge_m: 0.0,
                previous_position: 0.0,
                velocity: 0.0,
                previous_velocity: 0.0,
                distance_travelled_m: 0.0,
                fuel_used_l: 0.0,
                co2_emitted_g: 0.0,
                intersection_wait_start_time_s: None
            }
        );

        let path1 = fastest_path(&map, h2, w1);

        vehicles.push(
            Vehicle {
                id:1,
                spec,
                trip: TripRequest {
                    origin_id: 1,
                    destination_id: 3,
                    departure_time_s: 0,
                    return_time_s: None,
                },
                state : VehicleState::WaitingToDepart,
                current_node: h2,
                next_node: Some(i1),
                path : path1,
                path_index: 0,
                position_on_edge_m: 0.0,
                previous_position: 0.0,
                velocity: 0.0,
                previous_velocity: 0.0,
                distance_travelled_m: 0.0,
                fuel_used_l: 0.0,
                co2_emitted_g: 0.0,
                intersection_wait_start_time_s: None
            }
        );

        let mut sim = SimulationEngine::new(config, vehicles);
        sim.run();
    }
}
