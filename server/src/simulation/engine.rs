use std::collections::HashMap;

use petgraph::graph::{EdgeIndex, NodeIndex};

use crate::map::intersection::{JunctionController, MovementRequest, RoadRule};
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
    fn distance_between_vehicles(
        current_road: Road,
        current_vehicle: &Vehicle,
        ahead_vehicle: Option<Vehicle>,
    ) -> f32;
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

    fn step(&mut self) {
        //update feux tricolores
        let mut updates = Vec::new();
        for node_idx in self.map.graph.node_indices() {
             let incoming: Vec<u32> = self.map.graph.edges_directed(node_idx, petgraph::Direction::Incoming)
                  .map(|e| e.weight().id)
                  .collect();
             if !incoming.is_empty() {
                  updates.push((node_idx, incoming));
             }
        }
        
        for (node_idx, incoming) in updates {
             self.map.graph[node_idx].update_traffic_lights(self.time_step_s, &incoming);
        }

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
                    let distance_ahead = SimulationConfig::distance_between_vehicles(
                        current_road.clone(),
                        &self.vehicles[i],
                        ahead,
                    );

                    if current_road.length_m - distance_ahead >= self.vehicles[i].spec.length_m {
                        let vehicle = &mut self.vehicles[i];
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

                    // récupération de la règle applicable à l'intersection suivante
                    let next_node_idx = next_node.unwrap();
                    let intersection_node = &self.map.graph[next_node_idx];
                    
                    // 1) règle par défaut de la carte
                    let mut rule = intersection_node.rules.get(&current_road.id).copied().unwrap_or(RoadRule::Priority);

                    // 2) règle forcée (scénario de Test)
                    if let Some(forced) = vehicle.forced_rules.get(&next_node_idx) {
                       rule = *forced;
                    }

                    vehicle.velocity += self.time_step_s
                        * match ahead {
                            Some(v) => vehicle.compute_acceleration(
                                vehicle.position_on_edge_m - v.position_on_edge_m - v.spec.length_m,
                                v.previous_velocity,
                                current_road.speed_limit_ms as f32,
                                self.minimum_gap,
                                self.acceleration_exponent,
                            ),
                            None => {
                                //adaptation de la vitesse en fonction de la règle du prochain carrefour
                                match rule {
                                    RoadRule::Stop => {
                                         vehicle.compute_acceleration(
                                            vehicle.position_on_edge_m,
                                            0.0,
                                            current_road.speed_limit_ms as f32,
                                            0.1,//s'arrete 10cm avant intersection
                                            self.acceleration_exponent,
                                        )
                                    },
                                    RoadRule::Yield | RoadRule::Priority => {
                                         vehicle.compute_acceleration(
                                            10000.0,
                                            0.0,
                                            current_road.speed_limit_ms as f32,
                                            self.minimum_gap,
                                            self.acceleration_exponent,
                                        )
                                    }
                                }
                            },
                        };
                }
                VehicleState::AtIntersection => {
                    if self.vehicles[i].intersection_wait_start_time_s.is_none() {
                        self.vehicles[i].intersection_wait_start_time_s = Some(self.current_time);
                    }
                    if let Some(start_time) = self.vehicles[i].intersection_wait_start_time_s {
                         if self.current_time - start_time > self.time_step_s * 1.5 {
                             self.vehicles[i].velocity = 0.0;
                         }
                    }
                }
                VehicleState::Arrived => {}
            }
        }

        // 3) construction interractions avec junction controller
        let mut per_node: HashMap<NodeIndex, Vec<MovementRequest>> = HashMap::new();
        for idx in 0..self.vehicles.len() {
            let is_waiting = self.vehicles[idx].state == VehicleState::AtIntersection;
            let is_approaching = self.vehicles[idx].state == VehicleState::EnRoute && self.vehicles[idx].position_on_edge_m < 50.0;
            
            if !is_waiting && !is_approaching {
                continue;
            }

            let path_idx = self.vehicles[idx].path_index;
            if path_idx + 2 >= self.vehicles[idx].path.len() {
                if is_waiting { 
                    self.vehicles[idx].state = VehicleState::Arrived; 
                }
                continue;
            }

            let from = self.vehicles[idx].path[path_idx];
            let via = self.vehicles[idx].path[path_idx + 1];
            let to = self.vehicles[idx].path[path_idx + 2];

            //récupération infos route entrante et règles du carrefour
            let incoming_edge = self.map.graph.find_edge(from, via).unwrap();
            let incoming_road = &self.map.graph[incoming_edge];
            let intersection_node = &self.map.graph[via];
            
            // déteciton de la règle applicable
            let map_rule = intersection_node.rules.get(&incoming_road.id).copied().unwrap_or(RoadRule::Priority);
            let mut rule = map_rule;

            if let Some(forced) = self.vehicles[idx].forced_rules.get(&via) {
                rule = *forced;
            }

            if is_waiting && rule == RoadRule::Stop {
                let arrive_time = self.vehicles[idx]
                    .intersection_wait_start_time_s
                    .unwrap_or(self.current_time);
                
                if self.current_time - arrive_time < 3.0 { //doit attendre au moins 3 secondes au Stop
                    self.vehicles[idx].velocity = 0.0;
                    continue;
                }
            }

            let prev_intersection = &self.map.graph[from];
            let intersection = &self.map.graph[via];
            let next_intersection = &self.map.graph[to];

            let entry_angle = intersection.compute_road_angle(prev_intersection);
            
            let exit_angle = intersection.compute_road_angle(next_intersection);

            let arrival_time = self.vehicles[idx]
                .intersection_wait_start_time_s
                .unwrap_or(self.current_time);

            let light_color = intersection.traffic_lights.get(&incoming_road.id).copied();

            per_node.entry(via).or_default().push(MovementRequest {
                vehicle_index: idx,
                vehicle_id: self.vehicles[idx].id,
                to,
                entry_angle,
                exit_angle,
                arrival_time,
                rule,
                light_color,
            });
        }

        // 4) laisser chaque carrefour décider des véhicules autorisés
        for (intersection_node, requests) in per_node {
            // récupération des angles d'entrée de TOUTES les requêtes pour ce carrefour
            let intersection = &self.map.graph[intersection_node];
            let neighbor_indices: Vec<_> = self.map.graph.neighbors(intersection_node).collect();
            let mut all_entry_angles = Vec::new();
            
            for neighbor_idx in neighbor_indices {
                let neighbor = &self.map.graph[neighbor_idx];
                all_entry_angles.push(neighbor.compute_road_angle(intersection));
            }

            let allowed = JunctionController::authorized_indices(&requests, &all_entry_angles);

            for req_idx in allowed {
                let req = &requests[req_idx];
                
                if self.vehicles[req.vehicle_index].state != VehicleState::AtIntersection {
                    continue;
                }

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

                let dist = SimulationConfig::distance_between_vehicles(
                    next_road.clone(),
                    &self.vehicles[req.vehicle_index],
                    ahead,
                );

                if next_road.length_m - dist < self.vehicles[req.vehicle_index].spec.length_m {
                    continue;
                }

                let vehicle = &mut self.vehicles[req.vehicle_index];
                vehicle.state = VehicleState::EnRoute;
                vehicle.position_on_edge_m = next_road.length_m - dist;
                vehicle.path_index += 1;
                vehicle.current_node = vehicle.path[vehicle.path_index];
                vehicle.next_node = vehicle.path.get(vehicle.path_index + 1).copied();
                vehicle.intersection_wait_start_time_s = None;
            }
        }

        // 5) avancer les véhicules engagés sur leurs routes
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
                self.vehicles[i].position_on_edge_m -= self.vehicles[i].velocity * self.time_step_s;
                self.vehicles[i].velocity = self.vehicles[i].velocity.max(0.0);

                if self.vehicles[i].position_on_edge_m <= 0.5 {
                    self.vehicles[i].position_on_edge_m = 0.0;
                    self.vehicles[i].state = VehicleState::AtIntersection;
                    self.vehicles[i].intersection_wait_start_time_s = Some(self.current_time);
                    

                    let next_node_idx = self.vehicles[i].next_node.unwrap();
                    let current_road = self.map.graph.edge_weight(current_road_index).unwrap();
                    
                    let mut rule = self.map.graph[next_node_idx].rules.get(&current_road.id).copied().unwrap_or(RoadRule::Priority);
                    if let Some(forced) = self.vehicles[i].forced_rules.get(&next_node_idx) {
                        rule = *forced;
                    }
                    
                    if rule == RoadRule::Stop {
                        self.vehicles[i].velocity = 0.0;
                    }
                }
            }
        }
    }
}

