use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{Vehicle, VehicleState};
use petgraph::graph::EdgeIndex;
use std::collections::HashMap;

pub trait Simulation {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self;
    fn get_vehicle_ahead(
        &self,
        road_index: EdgeIndex,
        current_vehicle_id: u64,
        current_vehicle_position: f32,
    ) -> Option<Vehicle>;
    fn calculate_free_distance(
        current_vehicle_position: f32,
        ahead_vehicle: Option<Vehicle>,
    ) -> f32;
    fn run(&mut self);
    fn step(&mut self);
}

pub struct SimulationEngine {
    pub config: SimulationConfig,
    pub vehicles: Vec<Vehicle>,
    pub current_time: f32,
    pub vehicles_by_road: HashMap<EdgeIndex, Vec<Vehicle>>,
}

impl SimulationEngine {
    fn get_vehicle_ahead_internal(
        vehicles_by_road: &HashMap<EdgeIndex, Vec<Vehicle>>,
        road_index: EdgeIndex,
        current_vehicle_id: u64,
        current_vehicle_position: f32,
    ) -> Option<Vehicle> {
        let mut closest_ahead_vehicle: Option<Vehicle> = None;
        let mut closest_ahead_position: f32 = -1.0;

        if let Some(vehicles) = vehicles_by_road.get(&road_index) {
            for vehicle in vehicles {
                if vehicle.id != current_vehicle_id && vehicle.previous_position <= current_vehicle_position && closest_ahead_position < vehicle.previous_position {
                    closest_ahead_position = vehicle.previous_position;
                    closest_ahead_vehicle = Some(vehicle.clone());
                }
            }
        }

        closest_ahead_vehicle
    }
}

impl Simulation for SimulationEngine {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self {
        let current_time = config.start_time;
        Self {
            config,
            vehicles,
            current_time,
            vehicles_by_road: HashMap::new(),
        }
    }

    fn get_vehicle_ahead(
        &self,
        road_index: EdgeIndex,
        current_vehicle_id: u64,
        current_vehicle_position: f32,
    ) -> Option<Vehicle> {
        Self::get_vehicle_ahead_internal(
            &self.vehicles_by_road,
            road_index,
            current_vehicle_id,
            current_vehicle_position
        )
    }

    fn calculate_free_distance(
        current_vehicle_position: f32,
        ahead_vehicle: Option<Vehicle>,
    ) -> f32 {
        match ahead_vehicle {
            Some(v) => {
                let obstacle_position = v.previous_position + v.spec.length;
                current_vehicle_position - obstacle_position
            }
            None => current_vehicle_position,
        }
    }

    fn run(&mut self) {
        for vehicle in &mut self.vehicles {
            vehicle.update_path(&self.config.map);
        }

        while self.current_time < self.config.end_time {
            self.step();
            self.current_time += self.config.time_step;
        }
    }

    fn step(&mut self) {
        for vehicle in &mut self.vehicles {
            vehicle.previous_velocity = vehicle.velocity;
            vehicle.previous_position = vehicle.position_on_road;
        }

        for vehicle in &mut self.vehicles {
            match vehicle.state {
                VehicleState::WaitingToDepart => {
                    if vehicle.get_available_distance_ahead(&self.config.map) > vehicle.spec.length {
                        vehicle.position_on_road = vehicle.spec.length;
                        vehicle.state = VehicleState::OnRoad;

                        let current_road_index = vehicle.get_current_road(&self.config.map);
                        self.vehicles_by_road.entry(current_road_index).or_insert(Vec::new()).push(vehicle.clone());
                    }
                }

                VehicleState::OnRoad => {
                    let current_road_index = vehicle.get_current_road(&self.config.map);
                    let current_road = self.config.map.graph
                        .edge_weight(current_road_index)
                        .ok_or("Vehicle not in map")
                        .unwrap();

                    let vehicle_ahead = Self::get_vehicle_ahead_internal(&self.vehicles_by_road, current_road_index, vehicle.id, vehicle.position_on_road);
                    let vehicle_ahead_option = match vehicle_ahead {
                        Some(vehicle_ahead) => Some((
                            vehicle_ahead.previous_position - vehicle_ahead.spec.length - vehicle.position_on_road,
                            vehicle_ahead.previous_velocity,
                        )),
                        None => None,
                    };
                    let acceleration = vehicle.compute_acceleration(
                        current_road.speed_limit,
                        self.config.minimum_gap,
                        vehicle_ahead_option,
                    );

                    vehicle.velocity += acceleration.clamp(0.0, current_road.speed_limit) * self.config.time_step;

                    vehicle.position_on_road += vehicle.velocity * self.config.time_step;

                    if vehicle.position_on_road >= current_road.length {
                        vehicle.position_on_road = current_road.length;
                        vehicle.velocity = 0.0;
                        vehicle.previous_velocity = 0.0;
                        
                        if vehicle.path_index + 1 == vehicle.path.len() - 1 {
                            vehicle.state = VehicleState::Arrived;
                        } else {
                            vehicle.state = VehicleState::AtIntersection;
                        }

                        self.vehicles_by_road.get_mut(&current_road_index).unwrap().retain(|v| v.id != vehicle.id); 
                    }
                }

                VehicleState::AtIntersection => {
                    let next_node = vehicle.path[vehicle.path_index + 1];
                    let target_node = vehicle.path[vehicle.path_index + 2];
                    let next_road_index = self.config.map.graph.find_edge(next_node, target_node).unwrap();
                    let next_road = self.config.map.graph.edge_weight(next_road_index).unwrap();

                    if next_road.length > vehicle.spec.length {
                        
                        vehicle.position_on_road = vehicle.spec.length;
                        vehicle.previous_position = 0.0;
                        vehicle.path_index += 1;
                        vehicle.state = VehicleState::OnRoad;
                        
                        self.vehicles_by_road.entry(next_road_index).or_insert(Vec::new()).push(vehicle.clone());
                    }
                }

                VehicleState::Arrived => {}
            };
        }
    }
}
