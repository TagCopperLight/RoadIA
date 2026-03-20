use crate::map::intersection::IntersectionRules;
use crate::map::model::Map;
use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{Vehicle, VehicleState};
use crate::scoring;
use petgraph::graph::EdgeIndex;
use std::collections::HashMap;

pub trait Simulation {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self;
    fn run(&mut self);
    fn step(&mut self);
    fn get_score(&self) -> f32;
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
            Some(vehicle_ahead) => vehicle_ahead.previous_position - vehicle_ahead.length - vehicle.position_on_road,
            None => map.graph.edge_weight(vehicle.get_current_road(map)).unwrap().length - vehicle.position_on_road,
        }
    }

    fn build_proxies(&self) -> Vec<VehicleProxy> {
        self.vehicles.iter().map(|v| VehicleProxy {
            id: v.id,
            previous_position: v.previous_position,
            previous_velocity: v.previous_velocity,
            length: v.spec.length,
            road_index: if v.state == VehicleState::Arrived {
                None
            } else {
                Some(v.get_current_road(&self.config.map))
            },
        }).collect()
    }

    /// Registers an intersection request for the next intersection on the vehicle's path,
    /// if one exists. Should be called after the vehicle has entered a new road.
    fn register_next_intersection_request(
        map: &mut Map,
        vehicle: &Vehicle,
        road_id: u32,
        road_length: f32,
        road_speed_limit: f32,
        from_coords: (f32, f32),
    ) {
        if vehicle.path_index + 1 >= vehicle.path.len() - 1 {
            return;
        }

        let to_coords = {
            let node_idx = if vehicle.path_index + 2 < vehicle.path.len() {
                vehicle.path[vehicle.path_index + 2]
            } else {
                vehicle.trip.destination
            };
            let node = &map.graph[node_idx];
            (node.x, node.y)
        };

        let arrival_time = (road_length - vehicle.position_on_road).max(0.0) / road_speed_limit;
        let next_node = vehicle.get_next_node();
        let next_intersection = &mut map.graph[next_node];
        let rule = next_intersection.get_rule(road_id);
        next_intersection.request_intersection(vehicle.id, rule, arrival_time, from_coords, to_coords);
    }

    /// Computes the acceleration for a vehicle with no vehicle immediately ahead,
    /// taking intersection rules into account.
    fn compute_acceleration_without_vehicle_ahead(
        vehicle: &Vehicle,
        speed_limit: f32,
        road_length: f32,
        road_id: u32,
        minimum_gap: f32,
        map: &Map,
    ) -> f32 {
        // On the last road: head straight to destination, no intersection to yield to.
        if vehicle.path_index + 1 == vehicle.path.len() - 1 {
            return vehicle.compute_acceleration(speed_limit, minimum_gap, f32::INFINITY, 0.0);
        }

        let next_intersection = &map.graph[vehicle.get_next_node()];
        let rule = next_intersection.get_rule(road_id);
        let gap_to_intersection = (road_length - vehicle.position_on_road).max(0.001);

        match rule {
            IntersectionRules::Stop => {
                vehicle.compute_acceleration(speed_limit, minimum_gap, gap_to_intersection, 0.0)
            }
            _ => {
                if next_intersection.get_permission_to_enter(vehicle.id) {
                    vehicle.compute_acceleration(speed_limit, minimum_gap, f32::INFINITY, 0.0)
                } else {
                    vehicle.compute_acceleration(speed_limit, minimum_gap, gap_to_intersection, 0.0)
                }
            }
        }
    }

    fn handle_waiting_to_depart(
        config: &mut SimulationConfig,
        vehicles_by_road: &mut HashMap<EdgeIndex, Vec<usize>>,
        proxies: &mut Vec<VehicleProxy>,
        vehicle: &mut Vehicle,
        vehicle_index: usize,
    ) {
        let current_road_index = vehicle.get_current_road(&config.map);
        let vehicle_ahead = Self::get_vehicle_ahead(vehicles_by_road, proxies, current_road_index, vehicle.id, vehicle.position_on_road);

        if Self::get_available_distance_ahead(&config.map, vehicle, vehicle_ahead) <= vehicle.spec.length {
            return;
        }

        vehicle.position_on_road = vehicle.spec.length;
        vehicle.state = VehicleState::OnRoad;

        vehicles_by_road.entry(current_road_index).or_insert_with(Vec::new).push(vehicle_index);

        if let Some(proxy) = proxies.get_mut(vehicle_index) {
            proxy.previous_position = vehicle.position_on_road;
            proxy.road_index = Some(current_road_index);
        }

        let current_road = config.map.graph.edge_weight(current_road_index).unwrap().clone();
        let from_coords = {
            let node = &config.map.graph[vehicle.get_current_node()];
            (node.x, node.y)
        };

        Self::register_next_intersection_request(
            &mut config.map,
            vehicle,
            current_road.id,
            current_road.length,
            current_road.speed_limit,
            from_coords,
        );
    }

    /// Attempts to move the vehicle to the next road. Handles both arrival at the
    /// destination and mid-journey road transitions.
    fn try_advance_to_next_road(
        config: &mut SimulationConfig,
        vehicles_by_road: &mut HashMap<EdgeIndex, Vec<usize>>,
        proxies: &mut Vec<VehicleProxy>,
        vehicle: &mut Vehicle,
        vehicle_index: usize,
        current_road_index: EdgeIndex,
        _current_time: f32,
    ) {
        let v_id = vehicle.id;

        if vehicle.path_index + 1 == vehicle.path.len() - 1 {
            vehicle.state = VehicleState::Arrived;
            vehicle.path_index += 1;
            vehicle.arrived_at = Some(_current_time);
            if let Some(road_vehicles) = vehicles_by_road.get_mut(&current_road_index) {
                road_vehicles.retain(|&v_idx| proxies[v_idx].id != v_id);
            }
            return;
        }

        let next_road_index = {
            let u = vehicle.path[vehicle.path_index + 1];
            let v = vehicle.path[vehicle.path_index + 2];
            config.map.graph.find_edge(u, v).unwrap()
        };
        let new_road = config.map.graph.edge_weight(next_road_index).unwrap().clone();

        let can_enter = vehicles_by_road.get(&next_road_index).map_or(true, |indices| {
            indices.iter().all(|&idx| {
                let proxy = &proxies[idx];
                proxy.road_index != Some(next_road_index)
                    || proxy.previous_position - proxy.length >= vehicle.spec.length + 1.0
            })
        });

        let has_permission = config.map.graph[vehicle.path[vehicle.path_index + 1]]
            .get_permission_to_enter(v_id);

        if !can_enter || !has_permission {
            return;
        }

        let intersection_coordinates = {
            let node = &config.map.graph[vehicle.get_next_node()];
            (node.x, node.y)
        };

        if let Some(road_vehicles) = vehicles_by_road.get_mut(&current_road_index) {
            road_vehicles.retain(|&v_idx| proxies[v_idx].id != v_id);
        }

        {
            let intersection_node = &mut config.map.graph[vehicle.get_next_node()];
            intersection_node.remove_request(v_id);
        }

        vehicle.path_index += 1;
        vehicle.position_on_road = vehicle.spec.length;
        vehicle.previous_position = 0.0;

        vehicles_by_road.entry(next_road_index).or_insert_with(Vec::new).push(vehicle_index);

        if let Some(proxy) = proxies.get_mut(vehicle_index) {
            proxy.previous_position = vehicle.position_on_road;
            proxy.road_index = Some(next_road_index);
        }

        Self::register_next_intersection_request(
            &mut config.map,
            vehicle,
            new_road.id,
            new_road.length,
            new_road.speed_limit,
            intersection_coordinates,
        );
    }

    fn handle_on_road(
        config: &mut SimulationConfig,
        vehicles_by_road: &mut HashMap<EdgeIndex, Vec<usize>>,
        proxies: &mut Vec<VehicleProxy>,
        vehicle: &mut Vehicle,
        vehicle_index: usize,
        current_time: f32,
    ) {
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
            vehicle.position_on_road,
        );

        let acceleration = match vehicle_ahead {
            Some(vehicle_ahead) => vehicle.compute_acceleration(
                current_road.speed_limit,
                config.minimum_gap,
                vehicle_ahead.previous_position - vehicle_ahead.length - vehicle.position_on_road,
                vehicle_ahead.previous_velocity,
            ),
            None => Self::compute_acceleration_without_vehicle_ahead(
                vehicle,
                current_road.speed_limit,
                current_road.length,
                current_road.id,
                config.minimum_gap,
                &config.map,
            ),
        };

        vehicle.velocity = (vehicle.velocity + acceleration * config.time_step).clamp(0.0, current_road.speed_limit);
        vehicle.position_on_road += vehicle.velocity * config.time_step;

        if vehicle.position_on_road >= current_road.length {
            vehicle.position_on_road = current_road.length;
            vehicle.velocity = 0.0;
            vehicle.previous_velocity = 0.0;

            Self::try_advance_to_next_road(
                config,
                vehicles_by_road,
                proxies,
                vehicle,
                vehicle_index,
                current_road_index,
                current_time,
            );
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

    fn get_score(&self) -> f32 {
        scoring::compute_score(&self.vehicles, &self.config)
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

        let mut proxies = self.build_proxies();
        let mut need_print_score = true;

        for (vehicle_index, vehicle) in self.vehicles.iter_mut().enumerate() {
            match vehicle.state {
                VehicleState::WaitingToDepart => {
                    Self::handle_waiting_to_depart(
                        &mut self.config,
                        &mut self.vehicles_by_road,
                        &mut proxies,
                        vehicle,
                        vehicle_index,
                    );
                    need_print_score = false;
                },
                VehicleState::OnRoad => {
                    scoring::update_co2_emissions(vehicle, self.config.time_step, self.config.air_density, self.config.gravity_coefficient);
                    Self::handle_on_road(
                        &mut self.config,
                        &mut self.vehicles_by_road,
                        &mut proxies,
                        vehicle,
                        vehicle_index,
                        self.current_time,
                    );
                    need_print_score = false;
                },
                VehicleState::Arrived => {
                }
            }
        }

        if need_print_score {
            println!("Score : {}", self.get_score());
        }
    }
}
