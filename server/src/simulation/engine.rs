use std::collections::HashMap;

use petgraph::graph::{EdgeIndex, NodeIndex};

use crate::map::intersection::{JunctionController, MovementRequest};
use crate::map::model::Map;
use crate::map::road::Road;
use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{Vehicle, VehicleState};

pub trait Simulation {
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
            map,
            start_time_s,
            end_time_s,
            time_step_s,
            vehicles,
            current_time: start_time_s,
            minimum_gap,
            acceleration_exponent,
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
                VehicleState::EnRoute if vehicle.id != current_vehicle_id => {
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
        closest_ahead_vehicle
    }

    fn run(&mut self) {
        while self.current_time < self.end_time_s {
            self.step();
            self.current_time += self.time_step_s;
        }
    }

    fn step(&mut self) {
        // 1) Snapshot des vitesses pour l'IDM
        for vehicle in &mut self.vehicles {
            vehicle.previous_velocity = vehicle.velocity;
        }

        // 2) Mise à jour des vitesses / états simples
        let vehicles_len = self.vehicles.len();
        for i in 0..vehicles_len {
            match self.vehicles[i].state {
                VehicleState::WaitingToDepart => {
                    let (current_node, next_node, _spec_length, vid) = {
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
                    let free = free_space_from_start(current_road, ahead.as_ref());

                    if free >= self.vehicles[i].spec.length_m {
                        let vehicle = &mut self.vehicles[i];
                        vehicle.state = VehicleState::EnRoute;
                        vehicle.position_on_edge_m = current_road.length_m;
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
                    if self.vehicles[i].intersection_wait_start_time_s.is_none() {
                        self.vehicles[i].intersection_wait_start_time_s = Some(self.current_time);
                    }
                }
                VehicleState::Arrived => {}
            }
        }

        // 3) Construire les demandes d'engagement pour chaque carrefour
        let mut per_node: HashMap<NodeIndex, Vec<MovementRequest>> = HashMap::new();
        for idx in 0..self.vehicles.len() {
            if self.vehicles[idx].state != VehicleState::AtIntersection {
                continue;
            }

            let path_idx = self.vehicles[idx].path_index;
            if path_idx + 2 >= self.vehicles[idx].path.len() {
                self.vehicles[idx].state = VehicleState::Arrived;
                continue;
            }

            let from = self.vehicles[idx].path[path_idx];
            let via = self.vehicles[idx].path[path_idx + 1];
            let to = self.vehicles[idx].path[path_idx + 2];

            let prev_intersection = &self.map.graph[from];
            let intersection = &self.map.graph[via];
            let next_intersection = &self.map.graph[to];

            let entry_angle = prev_intersection.compute_road_angle(intersection);
            let exit_angle = intersection.compute_road_angle(next_intersection);

            let arrival_time = self.vehicles[idx]
                .intersection_wait_start_time_s
                .unwrap_or(self.current_time);

            per_node.entry(via).or_default().push(MovementRequest {
                vehicle_index: idx,
                vehicle_id: self.vehicles[idx].id,
                to,
                entry_angle,
                exit_angle,
                arrival_time,
            });
        }

        // 4) Laisser chaque carrefour décider des véhicules autorisés
        for (intersection_node, requests) in per_node {
            // Récupérer les angles de toutes les routes entrantes
            let intersection = &self.map.graph[intersection_node];
            let neighbor_indices: Vec<_> = self.map.graph.neighbors(intersection_node).collect();
            let mut all_entry_angles = Vec::new();
            
            for neighbor_idx in neighbor_indices {
                let neighbor = &self.map.graph[neighbor_idx];
                // L'angle d'entrée est l'angle VENANT du voisin VERS le centre
                // Soit compute_road_angle(neighbor, intersection)
                all_entry_angles.push(neighbor.compute_road_angle(intersection));
            }

            let allowed = JunctionController::authorized_indices(&requests, &all_entry_angles);

            for req_idx in allowed {
                let req = &requests[req_idx];

                let Some(next_road_index) = self.map.graph.find_edge(intersection_node, req.to)
                else {
                    continue;
                };
                let next_road = self.map.graph.edge_weight(next_road_index).unwrap();

                let ahead = {
                    let vehicles_imm = &self.vehicles;
                    SimulationConfig::ahead_vehicle(
                        vehicles_imm,
                        &self.map,
                        next_road_index,
                        req.vehicle_id,
                        next_road.length_m,
                    )
                };

                let free = free_space_from_start(next_road, ahead.as_ref());
                if free < self.vehicles[req.vehicle_index].spec.length_m {
                    continue;
                }

                let vehicle = &mut self.vehicles[req.vehicle_index];
                vehicle.state = VehicleState::EnRoute;
                vehicle.position_on_edge_m = next_road.length_m;
                vehicle.path_index += 1;
                vehicle.current_node = vehicle.path[vehicle.path_index];
                vehicle.next_node = vehicle.path.get(vehicle.path_index + 1).copied();
                vehicle.intersection_wait_start_time_s = None;
            }
        }

        // 5) Avancer les véhicules engagés sur leurs routes
        let vehicles_len = self.vehicles.len();
        for i in 0..vehicles_len {
            if self.vehicles[i].state == VehicleState::EnRoute {
                let current_road_index = self
                    .map
                    .graph
                    .find_edge(
                        self.vehicles[i].current_node,
                        self.vehicles[i].next_node.unwrap(),
                    )
                    .unwrap();
                let _current_road = self.map.graph.edge_weight(current_road_index).unwrap();
                self.vehicles[i].position_on_edge_m -= self.vehicles[i].velocity;
                self.vehicles[i].velocity = self.vehicles[i].velocity.max(0.0);
                if self.vehicles[i].position_on_edge_m <= self.vehicles[i].spec.length_m {
                    self.vehicles[i].position_on_edge_m = 0.0;
                    self.vehicles[i].state = VehicleState::AtIntersection;
                    self.vehicles[i].intersection_wait_start_time_s = Some(self.current_time);
                }
            }
        }
    }
}

fn free_space_from_start(current_road: &Road, ahead_vehicle: Option<&Vehicle>) -> f32 {
    match ahead_vehicle {
        Some(v) => (current_road.length_m - (v.position_on_edge_m + v.spec.length_m)).max(0.0),
        None => current_road.length_m,
    }
}
