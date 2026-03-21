use crate::simulation::config::{ACCELERATION_EXPONENT, MAX_SPEED};
use petgraph::graph::{EdgeIndex, NodeIndex};

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
    //Les attributs suivant permettent de calculer les émissions co2
    pub mass : f32,//en kg
    pub engine_thermal_efficiency: f32,
    pub drive_train_efficiency: f32,
    pub idle_power: f32, // en W
    pub lower_heating_value_for_fuel: f32,//en Kj/Kg
    pub aerodynamic_drag_coefficient: f32,
    pub front_area: f32,
    pub rolling_resistance_coefficient: f32,
    pub stoichiometric_co2_factor: f32
}

impl VehicleSpec {
    pub fn new(kind: VehicleKind, max_speed: f32, max_acceleration: f32, comfortable_deceleration: f32, reaction_time: f32, length: f32) -> Self {
        Self {
            kind,
            max_speed,
            max_acceleration,
            comfortable_deceleration,
            reaction_time,
            length,
            mass: 1680.0,
            engine_thermal_efficiency: 0.35,
            drive_train_efficiency: 0.9,
            idle_power: 2500.0,
            lower_heating_value_for_fuel: 43200.0,
            aerodynamic_drag_coefficient: 0.3,
            front_area: 2.0,
            rolling_resistance_coefficient: 0.01,
            stoichiometric_co2_factor: 3.16,
        }
    }
}

#[derive(Clone)]
pub struct TripRequest {
    pub origin: NodeIndex,
    pub destination: NodeIndex,
    pub departure_time: u64,
    pub return_time: Option<u64>,
}

#[derive(Copy, Clone, PartialEq)]
pub enum VehicleState {
    WaitingToDepart,
    OnRoad,
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
    pub emitted_co2 : f32, //en g de Co2
    pub arrived_at : Option<f32>
}

pub fn fastest_path(map: &Map, source: NodeIndex, destination: NodeIndex) -> Vec<NodeIndex> {
    let result = petgraph::algo::astar(
        &map.graph,
        source,
        |finish| finish == destination,
        |e| e.weight().length / f32::from(e.weight().speed_limit),
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
            emitted_co2: 0.0,
            arrived_at: None
        }
    }

    pub fn update_path(&mut self, map: &Map) {
        self.path = fastest_path(map, self.trip.origin, self.trip.destination);
        self.path_index = 0;

        if self.path.len() < 2 {
            self.state = VehicleState::Arrived;
        }
    }

    pub fn compute_acceleration(
        &self,
        desired_velocity: f32,
        minimum_gap: f32,
        vehicle_ahead_distance: f32,
        vehicle_ahead_velocity: f32,
    ) -> f32 {
        let free_road_acc = self.spec.max_acceleration
            * (1.0 - (self.previous_velocity / desired_velocity).powf(ACCELERATION_EXPONENT));

        if vehicle_ahead_distance <= 0.0 {
            panic!("Vehicle ahead is too close");
        }
        let s: f32 = minimum_gap
            + self.previous_velocity * self.spec.reaction_time
                + 0.5 * self.previous_velocity * (self.previous_velocity - vehicle_ahead_velocity)
                    / (self.spec.max_acceleration * self.spec.comfortable_deceleration)
                        .powf(0.5);

        free_road_acc - self.spec.max_acceleration * (s / vehicle_ahead_distance).powf(2.0)
    }

    pub fn get_coordinates(&self, map: &Map) -> Coordinates {
        let current_node = map
            .graph
            .node_weight(self.get_current_node())
            .ok_or("Vehicle not in map")
            .unwrap();
        match self.state {
            VehicleState::OnRoad => {
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

                let pos_rate: f32 = self.position_on_road / current_road.length;
                Coordinates {
                    x: current_node.x * (1.0 - pos_rate) + next_node_o.x * pos_rate,
                    y: current_node.y * (1.0 - pos_rate) + next_node_o.y * pos_rate,
                }
            }
            _ => Coordinates {
                x: current_node.x,
                y: current_node.y,
            },
        }
    }

    pub fn get_current_node(&self) -> NodeIndex {
        self.path[self.path_index]
    }

    pub fn get_next_node(&self) -> NodeIndex {
        if self.path_index + 1 >= self.path.len() {
            panic!("Vehicle is at destination");
        }
        self.path[self.path_index + 1]
    }

    pub fn get_current_road(&self, map: &Map) -> EdgeIndex {
        map.graph
            .find_edge(self.get_current_node(), self.get_next_node())
            .ok_or("Edge not in map")
            .unwrap()
    }

}
