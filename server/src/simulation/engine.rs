use crate::map::model::Map;
use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{Vehicle, VehicleState};
use petgraph::graph::EdgeIndex;
use std::collections::HashMap;

pub trait Simulation {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self;
    fn run(&mut self);
    fn step(&mut self);
}

#[derive(Clone)]
struct VehicleProxy {
    id: u64,
    previous_position: f32,
    previous_velocity: f32,
    length: f32,
    road_index: Option<EdgeIndex>,
}

pub struct SimulationEngine {
    pub config: SimulationConfig,
    pub vehicles: Vec<Vehicle>,
    pub current_time: f32,
    pub vehicles_by_road: HashMap<EdgeIndex, Vec<usize>>,
}

impl SimulationEngine {
    fn get_vehicle_ahead(
        vehicles_by_road: &HashMap<EdgeIndex, Vec<usize>>,
        proxies: &[VehicleProxy],
        road_index: EdgeIndex,
        current_vehicle_id: u64,
        current_vehicle_position: f32,
    ) -> Option<VehicleProxy> {
        let mut closest_ahead_vehicle: Option<VehicleProxy> = None;
        let mut closest_ahead_position: f32 = f32::MAX;

        if let Some(vehicle_indices) = vehicles_by_road.get(&road_index) {
            for &vehicle_index in vehicle_indices {
                let vehicle = &proxies[vehicle_index];
                if vehicle.id != current_vehicle_id {
                    let effective_position = if vehicle.road_index == Some(road_index) {
                        vehicle.previous_position
                    } else {
                        vehicle.length
                    };

                    if effective_position > current_vehicle_position && effective_position < closest_ahead_position {
                        closest_ahead_position = effective_position;
                        closest_ahead_vehicle = Some(vehicle.clone());
                    }
                }
            }
        }

        closest_ahead_vehicle
    }

    fn get_available_distance_ahead(map: &Map, vehicle: &Vehicle, vehicle_ahead: Option<VehicleProxy>) -> f32 {
        match vehicle_ahead {
            Some(vehicle_ahead) => vehicle_ahead.previous_position - vehicle_ahead.length - vehicle.position_on_road,
            None => map.graph.edge_weight(vehicle.get_current_road(map)).unwrap().length - vehicle.position_on_road,
        }
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

        let mut proxies: Vec<VehicleProxy> = self.vehicles.iter().map(|v| VehicleProxy {
            id: v.id,
            previous_position: v.previous_position,
            previous_velocity: v.previous_velocity,
            length: v.spec.length,
            road_index: if v.state == VehicleState::Arrived {
                None
            } else {
                Some(v.get_current_road(&self.config.map))
            },
        }).collect();

        for (vehicle_index, vehicle) in self.vehicles.iter_mut().enumerate() {
            match vehicle.state {
                VehicleState::WaitingToDepart => {
                    let vehicle_ahead = Self::get_vehicle_ahead(&self.vehicles_by_road, &proxies, vehicle.get_current_road(&self.config.map), vehicle.id, vehicle.position_on_road);
                    if Self::get_available_distance_ahead(&self.config.map, vehicle, vehicle_ahead) > vehicle.spec.length {
                        vehicle.position_on_road = vehicle.spec.length;
                        vehicle.state = VehicleState::OnRoad;

                        let current_road_index = vehicle.get_current_road(&self.config.map);
                        self.vehicles_by_road.entry(current_road_index).or_insert(Vec::new()).push(vehicle_index);

                        if let Some(proxy) = proxies.get_mut(vehicle_index) {
                            proxy.previous_position = vehicle.position_on_road;
                            proxy.road_index = Some(current_road_index);
                        }
                    }
                }

                VehicleState::OnRoad => {
                    let current_road_index = vehicle.get_current_road(&self.config.map);
                    let current_road = self.config.map.graph
                        .edge_weight(current_road_index)
                        .ok_or("Vehicle not in map")
                        .unwrap();

                    let vehicle_ahead = Self::get_vehicle_ahead(&self.vehicles_by_road, &proxies, current_road_index, vehicle.id, vehicle.position_on_road);
                    let vehicle_ahead_option = match vehicle_ahead {
                        Some(vehicle_ahead) => Some((
                            vehicle_ahead.previous_position - vehicle_ahead.length - vehicle.position_on_road,
                            vehicle_ahead.previous_velocity,
                        )),
                        None => None,
                    };
                    let acceleration = vehicle.compute_acceleration(
                        current_road.speed_limit,
                        self.config.minimum_gap,
                        vehicle_ahead_option,
                    );

                    vehicle.velocity += acceleration * self.config.time_step;
                    vehicle.velocity = vehicle.velocity.clamp(0.0, current_road.speed_limit);

                    vehicle.position_on_road += vehicle.velocity * self.config.time_step;

                    if vehicle.position_on_road >= current_road.length {
                        vehicle.position_on_road = current_road.length;
                        vehicle.velocity = 0.0;
                        vehicle.previous_velocity = 0.0;
                        
                        if vehicle.path_index + 1 == vehicle.path.len() - 1 {
                            vehicle.state = VehicleState::Arrived;
                            vehicle.path_index += 1;
                        } else {
                            vehicle.state = VehicleState::AtIntersection;
                        }
                        
                        let v_id = vehicle.id;
                        if let Some(road_vehicles) = self.vehicles_by_road.get_mut(&current_road_index) {
                             road_vehicles.retain(|&v_idx| proxies[v_idx].id != v_id);
                        }
                    }
                }

                VehicleState::AtIntersection => {
                    let next_node = vehicle.path[vehicle.path_index + 1];
                    let target_node = vehicle.path[vehicle.path_index + 2];
                    let next_road_index = self.config.map.graph.find_edge(next_node, target_node).unwrap();

                    let vehicle_ahead = Self::get_vehicle_ahead(&self.vehicles_by_road, &proxies, next_road_index, vehicle.id, 0.0);
                    let available_distance = match vehicle_ahead {
                        Some(ahead) => ahead.previous_position - ahead.length,
                        None => self.config.map.graph.edge_weight(next_road_index).unwrap().length,
                    };

                    if available_distance >= vehicle.spec.length {
                        vehicle.position_on_road = vehicle.spec.length;
                        vehicle.previous_position = 0.0;
                        vehicle.path_index += 1;
                        vehicle.state = VehicleState::OnRoad;
                        
                        self.vehicles_by_road.entry(next_road_index).or_insert(Vec::new()).push(vehicle_index);

                        if let Some(proxy) = proxies.get_mut(vehicle_index) {
                            proxy.previous_position = vehicle.position_on_road;
                            proxy.road_index = Some(next_road_index);
                        }
                    }
                }

                VehicleState::Arrived => {}
            };
        }
    }
}
