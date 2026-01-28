use petgraph::graph::{EdgeIndex, Graph, NodeIndex};

use crate::map::model::Map;
use crate::map::road::Road;
use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{Vehicle, VehicleSpec, VehicleState};

trait Simulation {
    fn new(
        map: Map,
        start_time_s: f32,
        end_time_s: f32,
        time_step_s: f32,
        vehicles: Vec<Vehicle>,
        minimum_gap: f32,
        acceleration_exponent: f32,
    ) -> Self;
    fn ahead_vehicle(
        vehicles: &[Vehicle],
        map: &Map,
        road_index: EdgeIndex,
        current_vehicle_id: u64,
        current_vehicle_position: f32,
    ) -> Option<Vehicle>;
    fn distance_between_vehicles(
        current_road: Road,
        current_vehicle: &Vehicle,
        ahead_vehicle: Option<Vehicle>,
    ) -> f32;
    fn run(&mut self);
    fn step(&mut self);
}

impl Simulation for SimulationConfig {
    fn new(
        map: Map,
        start_time_s: f32,
        end_time_s: f32,
        time_step_s: f32,
        vehicles: Vec<Vehicle>,
        minimum_gap: f32,
        acceleration_exponent: f32,
    ) -> Self {
        Self {
            map: map,
            start_time_s: start_time_s,
            end_time_s: end_time_s,
            time_step_s: time_step_s,
            vehicles: vehicles,
            current_time: start_time_s,
            minimum_gap: minimum_gap,
            acceleration_exponent: acceleration_exponent,
        }
    }

    fn ahead_vehicle(
        vehicles: &[Vehicle],
        map: &Map,
        road_index: EdgeIndex,
        current_vehicle_id: u64,
        current_vehicle_position: f32,
    ) -> Option<Vehicle> {
        let mut closest_ahead_vehicle: Option<Vehicle> = None;
        let mut closest_ahead_position: f32 = 0.0;
        for vehicle in vehicles {
            match vehicle.state {
                VehicleState::EnRoute | VehicleState::AtIntersection
                    if vehicle.id != current_vehicle_id =>
                {
                    if let Some(edge_index) = map
                        .graph
                        .find_edge(vehicle.current_node, vehicle.next_node.unwrap())
                    {
                        if edge_index == road_index
                            && vehicle.position_on_edge_m <= current_vehicle_position
                            && closest_ahead_position < vehicle.position_on_edge_m
                        {
                            closest_ahead_position = vehicle.position_on_edge_m;
                            closest_ahead_vehicle = Some(vehicle.clone());
                        }
                    }
                }
                VehicleState::EnRoute | VehicleState::AtIntersection => {}
                VehicleState::WaitingToDepart | VehicleState::Arrived => {}
            }
        }
        return closest_ahead_vehicle;
    }

    fn distance_between_vehicles(
        current_road: Road,
        current_vehicle: &Vehicle,
        ahead_vehicle: Option<Vehicle>,
    ) -> f32 {
        match ahead_vehicle {
            Some(v) => current_vehicle.position_on_edge_m - v.position_on_edge_m + v.spec.length_m,
            None => current_road.length_m,
        }
    }

    fn run(&mut self) {
        while self.current_time < self.end_time_s {
            self.step();
            self.current_time += self.time_step_s;
        }
    }

    fn step(&mut self) {
        //MAJ des attributs tempons
        let vehicles_len = self.vehicles.len();
        let vehicles_slice = &mut self.vehicles;
        for i in 0..vehicles_len {
            let vehicle = &mut vehicles_slice[i];
            vehicle.previous_velocity = vehicle.velocity;
        }
        //MAJ des vitesses et des états

        let vehicles_len = self.vehicles.len();
        for i in 0..vehicles_len {
            let state = self.vehicles[i].state;
            match state {
                VehicleState::WaitingToDepart => {
                    let (current_node, next_node, spec_length, vid) = {
                        let v = &self.vehicles[i];
                        (v.current_node, v.next_node, v.spec.length_m, v.id)
                    };
                    let current_road_index = self
                        .map
                        .graph
                        .find_edge(current_node, next_node.unwrap())
                        .unwrap();
                    let current_road = self.map.graph.edge_weight(current_road_index).unwrap();
                    let ahead = {
                        let vehicles_imm = &self.vehicles;
                        SimulationConfig::ahead_vehicle(
                            vehicles_imm,
                            &self.map,
                            current_road_index,
                            vid,
                            current_road.length_m,
                        )
                    };
                    let vehicle = &mut self.vehicles[i];
                    let distance_ahead: f32 = SimulationConfig::distance_between_vehicles(
                        current_road.clone(),
                        vehicle,
                        ahead,
                    );
                    if current_road.length_m - distance_ahead >= vehicle.spec.length_m {
                        vehicle.state = VehicleState::EnRoute;
                        vehicle.position_on_edge_m = current_road.length_m - distance_ahead;
                    }
                }
                VehicleState::EnRoute => {
                    let (current_node, next_node, pos, vid) = {
                        let v = &self.vehicles[i];
                        (v.current_node, v.next_node, v.position_on_edge_m, v.id)
                    };
                    let current_road_index = self
                        .map
                        .graph
                        .find_edge(current_node, next_node.unwrap())
                        .unwrap();
                    let current_road = self.map.graph.edge_weight(current_road_index).unwrap();
                    let ahead = {
                        let vehicles_imm = &self.vehicles;
                        SimulationConfig::ahead_vehicle(
                            vehicles_imm,
                            &self.map,
                            current_road_index,
                            vid,
                            pos,
                        )
                    };
                    let vehicle = &mut self.vehicles[i];
                    vehicle.velocity += self.time_step_s
                        * match ahead {
                            Some(v) => vehicle.compute_acceleration(
                                vehicle.position_on_edge_m - v.position_on_edge_m,
                                v.previous_velocity,
                                current_road.speed_limit_ms as f32,
                                self.minimum_gap,
                                self.acceleration_exponent,
                            ),
                            None => vehicle.compute_acceleration(
                                vehicle.position_on_edge_m,
                                0.0,
                                current_road.speed_limit_ms as f32,
                                self.minimum_gap,
                                self.acceleration_exponent,
                            ),
                        };
                }
                VehicleState::AtIntersection => {
                    let (path_idx, vid) = {
                        let v = &self.vehicles[i];
                        (v.path_index, v.id)
                    };
                    let n1 = *{
                        let v = &self.vehicles[i];
                        v.path.get(path_idx + 1).unwrap()
                    };
                    let n2 = *{
                        let v = &self.vehicles[i];
                        v.path.get(path_idx + 2).unwrap()
                    };
                    let next_road_index = self.map.graph.find_edge(n1, n2);
                    let next_road = self
                        .map
                        .graph
                        .edge_weight(next_road_index.unwrap())
                        .unwrap();
                    let ahead = {
                        let vehicles_imm = &self.vehicles;
                        SimulationConfig::ahead_vehicle(
                            vehicles_imm,
                            &self.map,
                            next_road_index.unwrap(),
                            vid,
                            next_road.length_m,
                        )
                    };
                    let vehicle = &mut self.vehicles[i];
                    let distance_ahead: f32 = SimulationConfig::distance_between_vehicles(
                        next_road.clone(),
                        vehicle,
                        ahead,
                    );
                    if next_road.length_m - distance_ahead >= vehicle.spec.length_m {
                        vehicle.state = VehicleState::EnRoute;
                        vehicle.position_on_edge_m = next_road.length_m - vehicle.spec.length_m;
                        vehicle.path_index += 1;
                        vehicle.current_node = *vehicle.path.get(vehicle.path_index).unwrap();
                        vehicle.next_node = vehicle.path.get(vehicle.path_index + 1).copied();
                    }
                }
                VehicleState::Arrived => {}
            }
        }
        //MAJ des positions
        let vehicles_len = self.vehicles.len();
        let vehicles_slice = &mut self.vehicles;
        for i in 0..vehicles_len {
            let vehicle = &mut vehicles_slice[i];
            if vehicle.state == VehicleState::EnRoute {
                let current_road_index = self
                    .map
                    .graph
                    .find_edge(vehicle.current_node, vehicle.next_node.unwrap())
                    .unwrap();
                let current_road = self.map.graph.edge_weight(current_road_index).unwrap();
                vehicle.position_on_edge_m -= vehicle.velocity;
                if vehicle.position_on_edge_m < 0.0 {
                    vehicle.position_on_edge_m = 0.0;
                    vehicle.state = VehicleState::AtIntersection;
                }
            }
        }
    }
}
