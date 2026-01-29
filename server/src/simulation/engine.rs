use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{Vehicle, VehicleState};
use petgraph::graph::EdgeIndex;

pub trait Simulation {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self;
    fn vehicle_ahead(
        &mut self,
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
}

impl Simulation for SimulationEngine {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self {
        let current_time = config.start_time_s;
        Self {
            config,
            vehicles,
            current_time,
        }
    }

    fn vehicle_ahead(
        &mut self,
        road_index: EdgeIndex,
        current_vehicle_id: u64,
        current_vehicle_position: f32,
    ) -> Option<Vehicle> {
        let mut closest_ahead_vehicle: Option<Vehicle> = None;
        let mut closest_ahead_position: f32 = -1.0;
        for vehicle in self.vehicles {
            match vehicle.state {
                VehicleState::EnRoute | VehicleState::AtIntersection
                    if vehicle.id != current_vehicle_id =>
                {
                    if let Some(edge_index) = self
                        .config
                        .map
                        .graph
                        .find_edge(vehicle.get_current_node(), vehicle.get_next_node().unwrap())
                    {
                        if edge_index == road_index
                            && vehicle.previous_position <= current_vehicle_position
                            && closest_ahead_position < vehicle.previous_position
                        {
                            closest_ahead_position = vehicle.previous_position;
                            closest_ahead_vehicle = Some(vehicle.clone());
                        }
                    }
                }
                VehicleState::EnRoute
                | VehicleState::AtIntersection
                | VehicleState::WaitingToDepart
                | VehicleState::Arrived => {}
            }
        }
        closest_ahead_vehicle
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
        while self.current_time < self.config.end_time_s {
            self.step();
            self.current_time += self.config.time_step_s;
        }
    }

    fn step(&mut self) {
        for vehicle in &mut self.vehicles {
            vehicle.previous_velocity = vehicle.velocity;
            vehicle.previous_position = vehicle.position_on_road;
        }
        //MAJ des vitesses et des états

        let vehicles_len = self.vehicles.len();
        for i in 0..vehicles_len {
            let vehicle = &mut self.vehicles[i];
            let state = self.vehicles[i].state;
            match state {
                VehicleState::WaitingToDepart => {
                    let available_distance_ahead =
                        vehicle.get_available_distance_ahead(&self.config.map);
                    if available_distance_ahead >= vehicle.spec.length {
                        vehicle.position_on_road = vehicle.spec.length;
                        vehicle.state = VehicleState::EnRoute
                    }
                }
                VehicleState::EnRoute => {
                    let current_road = vehicle.get_current_road(&self.config.map);
                    let current_speed_limit_ms = current_road.speed_limit_ms as f32;
                    let vehicle_ahead =
                        self.vehicle_ahead(current_road, vehicle.id, vehicle.position_on_road);
                    let vehicle_ahead_option = match vehicle_ahead {
                        Some(vehicle_ahead) => Some((
                            vehicle_ahead.previous_position
                                - vehicle_ahead.spec.length
                                - vehicle.position_on_road,
                            vehicle_ahead.previous_velocity,
                        )),
                        None => None,
                    };
                    let acceleration = vehicle.compute_acceleration(
                        current_speed_limit_ms,
                        self.config.minimum_gap,
                        vehicle_ahead_option,
                    );
                    vehicle.velocity +=
                        self.config.time_step_s * acceleration.clamp(0.0, current_speed_limit_ms);

                    vehicle.position_on_road -= vehicle.velocity * self.config.time_step_s;

                    if vehicle.position_on_road < current_road.length_m {
                        vehicle.on_node_reached();
                    }
                }
                VehicleState::AtIntersection => {
                    let available_distance_ahead =
                        vehicle.get_available_distance_ahead(&self.config.map);
                    if available_distance_ahead >= vehicle.spec.length {
                        vehicle.enter_next_road();
                    }
                }
                VehicleState::Arrived => {}
            };
        }
    }
}
