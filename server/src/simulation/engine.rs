use petgraph::graph::EdgeIndex;

use crate::map::model::Map;
use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{Vehicle, VehicleState};

pub trait Simulation {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self;
    fn ahead_vehicle(
        vehicles: &[Vehicle],
        map: &Map,
        road_index: EdgeIndex,
        current_vehicle_id: u64,
        current_vehicle_position: f32,
    ) -> Option<Vehicle>;
    fn next_obstacle_position(
        current_vehicle_position: f32,
        ahead_vehicle: Option<Vehicle>,
    ) -> f32;
    fn run(&mut self);
    fn step(&mut self);
}

#[derive(Debug)]
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
                            && vehicle.previous_position <= current_vehicle_position
                            && closest_ahead_position < vehicle.position_on_edge_m
                        {
                            closest_ahead_position = vehicle.previous_position;
                            closest_ahead_vehicle = Some(vehicle.clone());
                        }
                    }
                }
                VehicleState::EnRoute | VehicleState::AtIntersection | VehicleState::WaitingToDepart | VehicleState::Arrived => {}
                }
        }
        closest_ahead_vehicle
    }

    fn next_obstacle_position(//renvoie la position de l'obstacle devant le plus proche (voiture / fin de route)
        current_vehicle_position: f32,
        ahead_vehicle: Option<Vehicle>,
    ) -> f32 {
        print!("[Distance ahead] ");
        match ahead_vehicle {
            Some(v) => {println!(": {}, {}, {}", current_vehicle_position, v.previous_position, v.spec.length_m);v.previous_position + v.spec.length_m},
            None => {println!(": {}", 0.0); 0.0},
        }
    }

    fn run(&mut self) {
        println!("Début de la simulation");
        while self.current_time < self.config.end_time_s {
            println!("Current time {}", self.current_time);
            self.step();
            self.current_time += self.config.time_step_s;
        }
    }

    fn step(&mut self) {
        //MAJ des attributs tempons
        for vehicle in &mut self.vehicles {
            vehicle.previous_velocity = vehicle.velocity;
            vehicle.previous_position = vehicle.position_on_edge_m;
        }
        //MAJ des vitesses et des états

        let vehicles_len = self.vehicles.len();
        for i in 0..vehicles_len {
            let state = self.vehicles[i].state;
            println!("Vehicle {:?} AT ({} / {:?}) {:?}", self.vehicles[i].id, self.vehicles[i].position_on_edge_m, self.vehicles[i].current_node, self.vehicles[i].state);
            match state {
                VehicleState::WaitingToDepart => {
                    let (current_node, next_node, vid) = {
                        let v = &self.vehicles[i];
                        (v.current_node, v.next_node, v.id)
                    };
                    let current_road_index = self
                        .config
                        .map
                        .graph
                        .find_edge(current_node, next_node.unwrap())
                        .unwrap();
                    let current_road = self
                        .config
                        .map
                        .graph
                        .edge_weight(current_road_index)
                        .unwrap();
                    let ahead = {
                        let vehicles_imm = &self.vehicles;
                        Self::ahead_vehicle(
                            vehicles_imm,
                            &self.config.map,
                            current_road_index,
                            vid,
                            current_road.length_m,
                        )
                    };
                    //println!("[WaitingForDepart] Current Road {}", current_road.id);
                    let vehicle = &mut self.vehicles[i];
                    let distance_ahead: f32 =
                        Self::next_obstacle_position(vehicle.previous_position, ahead);
                    //println!("Distance ahead : {} cond : {}", distance_ahead, current_road.length_m - distance_ahead >= vehicle.spec.length_m);
                    if current_road.length_m - distance_ahead >= vehicle.spec.length_m {
                        vehicle.state = VehicleState::EnRoute;
                        vehicle.position_on_edge_m = current_road.length_m - distance_ahead;
                    }
                }
                VehicleState::EnRoute => {
                    let (current_node, next_node, pos, vid) = {
                        let v = &self.vehicles[i];
                        (v.current_node, v.next_node, v.previous_position, v.id)
                    };
                    let current_road_index = self
                        .config
                        .map
                        .graph
                        .find_edge(current_node, next_node.unwrap())
                        .unwrap();
                    let current_road = self
                        .config
                        .map
                        .graph
                        .edge_weight(current_road_index)
                        .unwrap();
                    let ahead = {
                        let vehicles_imm = &self.vehicles;
                        Self::ahead_vehicle(
                            vehicles_imm,
                            &self.config.map,
                            current_road_index,
                            vid,
                            pos,
                        )
                    };
                    let vehicle = &mut self.vehicles[i];
                    let new_acceleration = match ahead {
                            Some(v) => vehicle.compute_acceleration_follower(
                                vehicle.previous_position - v.previous_position - v.spec.length_m,
                                v.previous_velocity,
                                current_road.speed_limit_ms as f32,
                                self.config.minimum_gap,
                                self.config.acceleration_exponent,
                            ),
                            None => vehicle.compute_acceleration_free_road(
                                current_road.speed_limit_ms as f32,
                                self.config.acceleration_exponent,
                            ),
                        };
                    vehicle.velocity += self.config.time_step_s
                        * new_acceleration;
                    //println!("");
                    //println!("Acceleration : {}", new_acceleration);
                    //println!("Velocity : {}", vehicle.velocity);
                    vehicle.position_on_edge_m -= vehicle.velocity * self.config.time_step_s;
                    if vehicle.position_on_edge_m < 0.0 {
                        vehicle.position_on_edge_m = 0.0;
                        vehicle.velocity = 0.0;
                        vehicle.previous_velocity = 0.0;
                        vehicle.state = VehicleState::AtIntersection;
                        if vehicle.path_index == vehicle.path.len() - 2{
                            vehicle.state = VehicleState::Arrived;
                        }
                    }
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
                    let next_road_index = self.config.map.graph.find_edge(n1, n2);
                    let next_road = self
                        .config
                        .map
                        .graph
                        .edge_weight(next_road_index.unwrap())
                        .unwrap();
                    let ahead = {
                        let vehicles_imm = &self.vehicles;
                        Self::ahead_vehicle(
                            vehicles_imm,
                            &self.config.map,
                            next_road_index.unwrap(),
                            vid,
                            next_road.length_m,
                        )
                    };
                    let vehicle = &mut self.vehicles[i];
                    let distance_ahead: f32 =
                        Self::next_obstacle_position(vehicle.previous_position, ahead);
                    println!("Distance ahead : {}", distance_ahead);
                    if next_road.length_m - distance_ahead >= vehicle.spec.length_m {
                        vehicle.state = VehicleState::EnRoute;
                        vehicle.position_on_edge_m = next_road.length_m - vehicle.spec.length_m;
                        vehicle.previous_position = vehicle.position_on_edge_m;
                        vehicle.path_index += 1;
                        vehicle.current_node = *vehicle.path.get(vehicle.path_index).unwrap();
                        vehicle.next_node = vehicle.path.get(vehicle.path_index + 1).copied();
                    }
                }
                VehicleState::Arrived => {}
            };
        }
    }
}
