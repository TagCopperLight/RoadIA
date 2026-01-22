use crate::map::model::Map;
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VehicleKind {
    Car,
    Bus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleSpec {
    pub kind: VehicleKind,
    pub max_speed_kmh: f32,
    pub length_m: f32,
    pub fuel_consumption_l_per_100km: f32,
    pub co2_g_per_km: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripRequest {
    pub origin_id: u64,
    pub destination_id: u64,
    pub departure_time_s: u32,
    pub return_time_s: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VehicleState {
    // Renamed from AgentState
    WaitingToDepart,
    EnRoute,
    AtIntersection,
    Arrived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vehicle {
    // Renamed from Agent
    pub id: u64,
    pub spec: VehicleSpec,
    pub trip: TripRequest,
    pub state: VehicleState,

    #[serde(skip)]
    pub current_node: NodeIndex,

    #[serde(skip)]
    pub next_node: Option<NodeIndex>,

    #[serde(skip)]
    pub path: Vec<NodeIndex>,

    #[serde(skip)]
    pub path_index: usize,

    pub position_on_edge_m: f32,

    pub x: f32,
    pub y: f32,

    pub departure_time_s: u32,
    pub arrival_time_s: Option<u32>,

    pub distance_travelled_m: f32,
    pub fuel_used_l: f32,
    pub co2_emitted_g: f32,

    #[serde(skip)]
    pub intersection_wait_start_time_s: Option<f32>,
}

// -----------------------------------------------------------------------------
// Routing
// -----------------------------------------------------------------------------

pub trait RoutingStrategy {
    fn compute_path(
        &self,
        map: &Map, // Renamed from network
        origin: NodeIndex,
        destination: NodeIndex,
        departure_time_s: u32,
    ) -> Vec<NodeIndex>;
}

pub struct ShortestPathStrategy;

impl RoutingStrategy for ShortestPathStrategy {
    fn compute_path(
        &self,
        map: &Map,
        origin: NodeIndex,
        destination: NodeIndex,
        _departure_time_s: u32,
    ) -> Vec<NodeIndex> {
        use petgraph::algo::astar;

        // Heuristique A* : distance euclidienne entre les nœuds
        let heuristic = |n: NodeIndex| {
            let node = &map.graph[n];
            let dest = &map.graph[destination];
            let dx = node.x - dest.x;
            let dy = node.y - dest.y;
            (dx * dx + dy * dy).sqrt()
        };

        // Coût réel : longueur de la route en mètres
        let edge_cost = |edge: petgraph::graph::EdgeReference<'_, crate::map::road::Road>| {
            edge.weight().length_m
        };

        // A*
        if let Some((_cost, path)) = astar(
            &map.graph,
            origin,
            |n| n == destination,
            edge_cost,
            heuristic,
        ) {
            println!(
                "[ROUTING A*] origin={:?} dest={:?} path={:?}",
                origin, destination, path
            );
            path
        } else {
            println!(
                "[ROUTING A*] No path found from {:?} to {:?}",
                origin, destination
            );
            Vec::new()
        }
    }
}

// -----------------------------------------------------------------------------
// Vehicle impl
// -----------------------------------------------------------------------------

impl Vehicle {
    pub fn new(
        id: u64,
        spec: VehicleSpec,
        trip: TripRequest,
        initial_node: NodeIndex,
        departure_time_s: u32,
        x: f32,
        y: f32,
    ) -> Self {
        Self {
            id,
            spec,
            trip,
            state: VehicleState::WaitingToDepart,
            current_node: initial_node,
            next_node: None,
            path: Vec::new(),
            path_index: 0,
            position_on_edge_m: 0.0,
            x,
            y,
            departure_time_s,
            arrival_time_s: None,
            distance_travelled_m: 0.0,
            fuel_used_l: 0.0,
            co2_emitted_g: 0.0,
            intersection_wait_start_time_s: None,
        }
    }
}
