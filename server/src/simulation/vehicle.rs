use crate::{
    map::road::Road,
    simulation::config::{ACCELERATION_EXPONENT, MAX_SPEED},
};
use petgraph::graph::NodeIndex;

use crate::map::{model::Coordinates, model::Map};

#[derive(Copy, Clone)]
pub enum VehicleKind {
    Car,
    Bus,
}

#[derive(Copy, Clone)]
pub struct VehicleSpec {
    pub kind: VehicleKind,
    pub max_speed: f32,
    pub max_acceleration: f32,
    pub comfortable_deceleration: f32,
    pub reaction_time: f32,
    pub length: f32,
}

#[derive(Clone)]
pub struct TripRequest {
    pub origin: NodeIndex,
    pub destination: NodeIndex,
    pub departure_time: u64,
    pub return_time: Option<u64>,
}

#[derive(Copy, Clone)]
pub enum VehicleState {
    WaitingToDepart,
    EnRoute,
    AtIntersection,
    Arrived,
}

#[derive(Clone)]
pub struct Vehicle {
    pub id: u64,
    pub spec: VehicleSpec,
    pub trip: TripRequest,
    pub state: VehicleState,

    pub path: Vec<NodeIndex>,
    pub path_index: usize,

    pub position_on_road: f32, // distance entre l'avant du véhicule et le début de la route
    pub previous_position: f32,
    pub velocity: f32,
    pub previous_velocity: f32,
}

pub fn fastest_path(map: &Map, source: NodeIndex, destination: NodeIndex) -> Vec<NodeIndex> {
    let result = petgraph::algo::astar(
        &map.graph,
        source,
        |finish| finish == destination,
        |e| e.weight().length_m / f32::from(e.weight().speed_limit_ms),
        |n| map.intersections_euclidean_distance(n, destination) / f32::from(MAX_SPEED),
    );
    match result {
        Some((_cost, path)) => path,
        None => Vec::new(),
    }
}

impl Vehicle {
    pub fn new(id: u64, spec: VehicleSpec, trip: TripRequest) -> Self {
        Self {
            id,
            spec,
            trip,
            state: VehicleState::WaitingToDepart,
            path: Vec::new(),
            path_index: 0,
            previous_velocity: 0.0,
            velocity: 0.0,
            position_on_road: 0.0,
            previous_position: 0.0,
        }
    }

    pub fn update_path(&mut self, map: &Map) {
        self.path = fastest_path(map, self.trip.origin, self.trip.destination);
    }

    pub fn compute_acceleration(
        &self,
        desired_velocity: f32,
        minimum_gap: f32,
        vehicle_ahead: Option<(f32, f32)>, // (distance, velocity)
    ) -> f32 {
        let free_road_acc = self.spec.max_acceleration
            * (1.0 - (self.previous_velocity / desired_velocity).powf(ACCELERATION_EXPONENT));

        match vehicle_ahead {
            Some((distance, velocity)) => {
                let s: f32 = minimum_gap
                    + self.previous_velocity * self.spec.reaction_time
                    + 0.5 * self.previous_velocity * (self.previous_velocity - velocity)
                        / (self.spec.max_acceleration * self.spec.comfortable_deceleration)
                            .powf(0.5);

                free_road_acc - self.spec.max_acceleration * (s / distance).powf(2.0)
            }
            None => free_road_acc,
        }
    }

    pub fn get_coordinates(&self, map: &Map) -> Coordinates {
        let current_node_o = map
            .graph
            .node_weight(self.get_current_node())
            .ok_or("Vehicle not in map")
            .unwrap();
        match self.state {
            VehicleState::EnRoute => {
                let next_node_o = map
                    .graph
                    .node_weight(self.get_next_node())
                    .ok_or("Vehicle not in map")
                    .unwrap();
                let current_road = map
                    .graph
                    .edge_weight(
                        map.graph
                            .find_edge(self.get_current_node(), self.get_next_node())
                            .ok_or("Edge not in map")
                            .unwrap(),
                    )
                    .ok_or("Edge not in map")
                    .unwrap();

                let pos_rate: f32 = self.position_on_road / current_road.length_m;
                Coordinates {
                    x: current_node_o.x * (1.0 - pos_rate) + next_node_o.x * pos_rate,
                    y: current_node_o.y * (1.0 - pos_rate) + next_node_o.y * pos_rate,
                }
            }
            _ => Coordinates {
                x: current_node_o.x,
                y: current_node_o.y,
            },
        }
    }

    pub fn get_current_node(&self) -> NodeIndex {
        self.path[self.path_index]
    }

    pub fn get_next_node(&self) -> NodeIndex {
        if self.path_index + 1 >= self.path.len() {
            return self.path[self.path_index];
        }
        self.path[self.path_index + 1]
    }

    pub fn get_current_road<'a>(&self, map: &'a Map) -> &'a Road {
        map.graph
            .edge_weight(
                map.graph
                    .find_edge(self.get_current_node(), self.get_next_node())
                    .ok_or("Edge not in map")
                    .unwrap(),
            )
            .ok_or("Edge not in map")
            .unwrap()
    }

    pub fn get_next_road<'a>(&self, map: &'a Map) -> &'a Road {
        map.graph
            .edge_weight(
                map.graph
                    .find_edge(self.get_next_node(), self.get_next_node())
                    .ok_or("Edge not in map")
                    .unwrap(),
            )
            .ok_or("Edge not in map")
            .unwrap()
    }

    pub fn on_node_reached(&mut self) {
        self.position_on_road = 0.0;
        self.velocity = 0.0;
        self.previous_velocity = 0.0;
        self.state = VehicleState::AtIntersection;
        if self.path_index == self.path.len() - 2 {
            self.state = VehicleState::Arrived;
        }
    }

    pub fn enter_next_road(&mut self) {
        self.state = VehicleState::EnRoute;
        self.position_on_road = self.spec.length;
        self.previous_position = self.position_on_road;
        self.path_index += 1;
    }

    pub fn get_available_distance_ahead(&self, map: &Map) -> f32 {
        let current_road = self.get_current_road(map);
        current_road.length_m - self.position_on_road
    }
}
