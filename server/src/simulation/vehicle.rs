use crate::map::intersection::Intersection;
use crate::map::model::Coordinates;
use crate::{map::model::Map, simulation::config::MAX_SPEED_MS};
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy)]
pub enum VehicleKind {
    Car,
    Bus,
}

#[derive(Debug, Clone)]
pub struct VehicleSpec {
    pub kind: VehicleKind,
    pub max_speed_ms: f32,
    pub max_acceleration_ms2: f32,
    pub comfortable_deceleration: f32,
    pub reaction_time: f32,
    pub length_m: f32,
    pub fuel_consumption_l_per_100km: f32,
    pub co2_g_per_km: f32,
}

#[derive(Debug, Clone)]
pub struct TripRequest {
    pub origin_id: u64,
    pub destination_id: u64,
    pub departure_time_s: u32,
    pub return_time_s: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VehicleState {
    WaitingToDepart,
    EnRoute,
    AtIntersection,
    Arrived,
}

#[derive(Debug, Clone)]
pub struct Vehicle {
    pub id: u64,
    pub spec: VehicleSpec,
    pub trip: TripRequest,
    pub state: VehicleState,

    pub current_node: NodeIndex,

    pub next_node: Option<NodeIndex>,

    // Override rule for SPECIFIC intersections (NodeIndex -> Rule)
    pub forced_rules: HashMap<NodeIndex, crate::map::intersection::RoadRule>,

    pub path: Vec<NodeIndex>,

    pub path_index: usize,

    pub position_on_edge_m: f32, //distance entre l'avant du véhicule et la fin de la route
    pub velocity: f32,
    pub previous_velocity: f32,

    pub distance_travelled_m: f32,
    pub fuel_used_l: f32,
    pub co2_emitted_g: f32,

    pub intersection_wait_start_time_s: Option<f32>,
}

// -----------------------------------------------------------------------------
// Routing
// -----------------------------------------------------------------------------

pub fn intersections_euclidean_distance(
    map: &Map,
    source: NodeIndex,
    destination: NodeIndex,
) -> f32 {
    let n1 = &map.graph[source];
    let n2 = &map.graph[destination];
    ((n1.x - n2.x).powf(2.0) + (n1.y - n2.y).powf(2.0)).sqrt()
}
pub fn fastest_path(map: &Map, source: NodeIndex, destination: NodeIndex) -> Vec<NodeIndex> {
    let result = petgraph::algo::astar(
        &map.graph,
        source,
        |finish| finish == destination,
        |e| e.weight().length_m / (e.weight().speed_limit_ms as f32),
        |n| intersections_euclidean_distance(map, n, destination) / (MAX_SPEED_MS as f32),
    );

    match result {
        Some((_cost, path)) => path,
        None => Vec::new(),
    }
}

// -----------------------------------------------------------------------------
// Vehicle impl
// -----------------------------------------------------------------------------

impl Vehicle {
    pub fn new(id: u64, spec: VehicleSpec, trip: TripRequest, initial_node: NodeIndex) -> Self {
        Self {
            id,
            spec,
            trip,
            state: VehicleState::WaitingToDepart,
            current_node: initial_node,
            next_node: None,
            path: Vec::new(),
            path_index: 0,
            previous_velocity: 0.0,
            velocity: 0.0,
            position_on_edge_m: 0.0,
            distance_travelled_m: 0.0,
            fuel_used_l: 0.0,
            co2_emitted_g: 0.0,
            intersection_wait_start_time_s: None,
            forced_rules: HashMap::new(),
        }
    }

    pub fn compute_acceleration(
        &self,
        b2b_distance: f32,
        next_vehicle_velocity: f32,
        desired_velocity: f32,
        minimum_gap: f32,
        acceleration_exponent: f32,
    ) -> f32 {
        let s: f32 = minimum_gap
            + self.previous_velocity * self.spec.reaction_time
            + 0.5 * self.previous_velocity * (self.previous_velocity - next_vehicle_velocity)
                / (self.spec.max_acceleration_ms2 * self.spec.comfortable_deceleration).powf(0.5);
        let new_acceleration: f32 = self.spec.max_acceleration_ms2
            * (1.0
                - (self.previous_velocity / desired_velocity).powf(acceleration_exponent)
                - (s / b2b_distance));
        return new_acceleration;
    }

    pub fn get_coordinates(&self, map: &Map) -> Coordinates {
        match self.state {
            VehicleState::WaitingToDepart => {
                let current_node_o = map.graph.node_weight(self.current_node).unwrap();
                return Coordinates {
                    x: current_node_o.x,
                    y: current_node_o.y,
                };
            }
            VehicleState::AtIntersection | VehicleState::EnRoute => {
                let current_node_o = map.graph.node_weight(self.current_node).unwrap();
                let next_node_o = map.graph.node_weight(self.next_node.unwrap()).unwrap();
                let current_road = map
                    .graph
                    .edge_weight(
                        map.graph
                            .find_edge(self.current_node, self.next_node.unwrap())
                            .unwrap(),
                    )
                    .unwrap();

                // Correct interpolation: pos starts at current_node (L) -> next_node (0)
                // pos_rate 0.0 -> Start (current), 1.0 -> End (next)
                let pos_rate: f32 = (1.0 - (self.position_on_edge_m / current_road.length_m)).min(1.0).max(0.0);
                
                return Coordinates {
                    x: current_node_o.x + (next_node_o.x - current_node_o.x) * pos_rate,
                    y: current_node_o.y + (next_node_o.y - current_node_o.y) * pos_rate,
                };
            }
            VehicleState::Arrived => {
                let current_node_o: Intersection = map
                    .graph
                    .node_weight(*self.path.get(self.path.len() - 1).unwrap())
                    .unwrap()
                    .clone();
                return Coordinates {
                    x: current_node_o.x,
                    y: current_node_o.y,
                };
            }
        }
    }
}