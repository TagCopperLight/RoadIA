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
        let mut closest_ahead: Option<VehicleProxy> = None;
        let mut closest_ahead_position: f32 = f32::MAX;

        if let Some(vehicle_indices) = vehicles_by_road.get(&road_index) {
            for &vehicle_index in vehicle_indices {
                let proxy = &proxies[vehicle_index];
                if proxy.id == current_vehicle_id {
                    continue;
                }

                let effective_position = if proxy.road_index == Some(road_index) {
                    proxy.previous_position
                } else {
                    proxy.length
                };

                if effective_position > current_vehicle_position && effective_position < closest_ahead_position {
                    closest_ahead_position = effective_position;
                    closest_ahead = Some(proxy.clone());
                }
            }
        }

        closest_ahead
    }

    fn get_available_distance_ahead(map: &Map, vehicle: &Vehicle, vehicle_ahead: Option<VehicleProxy>) -> f32 {
        match vehicle_ahead {
            Some(vehicle_ahead) => vehicle_ahead.previous_position - vehicle_ahead.length - vehicle.position_on_lane,
            None => map.graph.edge_weight(vehicle.get_current_road(map)).unwrap().length - vehicle.position_on_lane,
        }
    }

    fn build_proxies(&self) -> Vec<VehicleProxy> {
        self.vehicles.iter().map(|v| VehicleProxy {
            id: v.id,
            previous_position: v.position_on_lane,
            previous_velocity: v.previous_velocity,
            length: v.spec.length,
            road_index: if v.state == VehicleState::Arrived {
                None
            } else {
                Some(v.get_current_road(&self.config.map))
            },
        }).collect()
    }

    fn compute_acceleration_without_vehicle_ahead(
        vehicle: &Vehicle,
        speed_limit: f32,
    ) -> f32 {
        vehicle.compute_acceleration(speed_limit, 0.0, f32::INFINITY, 0.0)
    }

    fn handle_waiting_to_depart(
        config: &mut SimulationConfig,
        vehicles_by_road: &mut HashMap<EdgeIndex, Vec<usize>>,
        proxies: &mut Vec<VehicleProxy>,
        vehicle: &mut Vehicle,
        vehicle_index: usize,
    ) {
        let current_road_index = vehicle.get_current_road(&config.map);
        let vehicle_ahead = Self::get_vehicle_ahead(vehicles_by_road, proxies, current_road_index, vehicle.id, vehicle.position_on_lane);

        if Self::get_available_distance_ahead(&config.map, vehicle, vehicle_ahead) <= vehicle.spec.length {
            return;
        }

        vehicle.position_on_lane = vehicle.spec.length;
        vehicle.state = VehicleState::OnRoad;

        vehicles_by_road.entry(current_road_index).or_insert_with(Vec::new).push(vehicle_index);

        if let Some(proxy) = proxies.get_mut(vehicle_index) {
            proxy.previous_position = vehicle.position_on_lane;
            proxy.road_index = Some(current_road_index);
        }

    }

    fn try_advance_to_next_road(
        config: &mut SimulationConfig,
        vehicles_by_road: &mut HashMap<EdgeIndex, Vec<usize>>,
        proxies: &mut Vec<VehicleProxy>,
        vehicle: &mut Vehicle,
        vehicle_index: usize,
        current_road_index: EdgeIndex,
    ) -> bool {
        let v_id = vehicle.id;

        if vehicle.path_index + 1 == vehicle.path.len() - 1 {
            vehicle.state = VehicleState::Arrived;
            vehicle.path_index += 1;
            if let Some(road_vehicles) = vehicles_by_road.get_mut(&current_road_index) {
                road_vehicles.retain(|&v_idx| proxies[v_idx].id != v_id);
            }
            return true;
        }

        let next_road_index = {
            let u = vehicle.path[vehicle.path_index + 1];
            let v = vehicle.path[vehicle.path_index + 2];
            config.map.graph.find_edge(u, v).unwrap()
        };

        let road_space_ok = vehicles_by_road.get(&next_road_index).map_or(true, |indices| {
            indices.iter().all(|&idx| {
                let proxy = &proxies[idx];
                proxy.road_index != Some(next_road_index)
                    || proxy.previous_position - proxy.length >= vehicle.spec.length + 1.0
            })
        });

        if !road_space_ok {
            return false;
        }

        if let Some(road_vehicles) = vehicles_by_road.get_mut(&current_road_index) {
            road_vehicles.retain(|&v_idx| proxies[v_idx].id != v_id);
        }

        vehicle.path_index += 1;
        vehicle.position_on_lane = vehicle.spec.length;

        vehicles_by_road.entry(next_road_index).or_insert_with(Vec::new).push(vehicle_index);

        if let Some(proxy) = proxies.get_mut(vehicle_index) {
            proxy.previous_position = vehicle.position_on_lane;
            proxy.road_index = Some(next_road_index);
        }

        true
    }

    fn handle_on_road(
        config: &mut SimulationConfig,
        vehicles_by_road: &mut HashMap<EdgeIndex, Vec<usize>>,
        proxies: &mut Vec<VehicleProxy>,
        vehicle: &mut Vehicle,
        vehicle_index: usize,
    ) {
        println!("Vehicle {} is on road {:?}", vehicle.id, vehicle.get_current_road(&config.map));
        let current_road_index = vehicle.get_current_road(&config.map);
        let current_road = config.map.graph
            .edge_weight(current_road_index)
            .ok_or("Vehicle not in map")
            .unwrap()
            .clone();

        let vehicle_ahead = Self::get_vehicle_ahead(
            vehicles_by_road,
            proxies,
            current_road_index,
            vehicle.id,
            vehicle.position_on_lane,
        );

        if let Some(v_ahead) = &vehicle_ahead {
            println!("Vehicle ahead: {}", v_ahead.id);
        } else {
            println!("No vehicle ahead");
        }

        let acceleration = match vehicle_ahead {
            Some(vehicle_ahead) => {
                println!("Vehicle {} calling compute_acceleration (WITH vehicle ahead {})", vehicle.id, vehicle_ahead.id);
                vehicle.compute_acceleration(
                    current_road.speed_limit,
                    config.minimum_gap,
                    vehicle_ahead.previous_position - vehicle_ahead.length - vehicle.position_on_lane,
                    vehicle_ahead.previous_velocity,
                )
            },
            None => {
                println!("Vehicle {} calling compute_acceleration_without_vehicle_ahead (WITHOUT vehicle ahead)", vehicle.id);
                println!("Speed limit {}, Length {}, Road id {}, Minimum gap {}", current_road.speed_limit, current_road.length, current_road.id, config.minimum_gap);
                Self::compute_acceleration_without_vehicle_ahead(
                    vehicle,
                    current_road.speed_limit,
                )
            },
        };

        
        vehicle.velocity = (vehicle.velocity + acceleration * config.time_step).clamp(0.0, current_road.speed_limit);
        vehicle.position_on_lane += vehicle.velocity * config.time_step;
        
        println!("Acceleration: {}", acceleration);
        println!("Velocity: {}", vehicle.velocity);
        println!("Position: {}", vehicle.position_on_lane);
        println!("It's {}% done with the road", (vehicle.position_on_lane / current_road.length) * 100.0);

        if vehicle.position_on_lane >= current_road.length - 1e-2 {
            vehicle.position_on_lane = current_road.length;

            let advanced = Self::try_advance_to_next_road(
                config,
                vehicles_by_road,
                proxies,
                vehicle,
                vehicle_index,
                current_road_index,
            );

            // If the vehicle could not advance, stop it at the road end.
            if !advanced {
                vehicle.velocity = 0.0;
                vehicle.previous_velocity = 0.0;
            }
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
        }

        let mut proxies = self.build_proxies();

        for (vehicle_index, vehicle) in self.vehicles.iter_mut().enumerate() {
            match vehicle.state {
                VehicleState::WaitingToDepart => Self::handle_waiting_to_depart(
                    &mut self.config,
                    &mut self.vehicles_by_road,
                    &mut proxies,
                    vehicle,
                    vehicle_index,
                ),
                VehicleState::OnRoad => Self::handle_on_road(
                    &mut self.config,
                    &mut self.vehicles_by_road,
                    &mut proxies,
                    vehicle,
                    vehicle_index,
                ),
                VehicleState::Arrived => {}
            }
        }
    }
}
