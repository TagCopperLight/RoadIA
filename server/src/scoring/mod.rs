use crate::map::model::Map;
use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{Vehicle, VehicleState};
use petgraph::graph::EdgeIndex;
use crate::map::intersection::IntersectionKind;
use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;

// Typical passenger car constants for CO2 estimation
const VEHICLE_MASS: f32 = 1680.0; // kg
const ENGINE_THERMAL_EFFICIENCY: f32 = 0.35;
const DRIVE_TRAIN_EFFICIENCY: f32 = 0.9;
const IDLE_POWER: f32 = 2500.0; // W
const LOWER_HEATING_VALUE_FOR_FUEL: f32 = 43200.0; // kJ/kg
const AERODYNAMIC_DRAG_COEFFICIENT: f32 = 0.3;
const FRONT_AREA: f32 = 2.0; // m²
const ROLLING_RESISTANCE_COEFFICIENT: f32 = 0.01;
const STOICHIOMETRIC_CO2_FACTOR: f32 = 3.16;

// Physics
const AIR_DENSITY: f32 = 1.225; // kg/m³
const GRAVITY: f32 = 9.81; // m/s²

// Score weights (must sum to 1.0)
const TIME_WEIGHT: f32 = 0.4;
const SUCCESS_WEIGHT: f32 = 0.2;
const POLLUTION_WEIGHT: f32 = 0.2;
const INFRASTRUCTURE_WEIGHT: f32 = 0.2;

pub fn get_minimal_time_travel_by_road(map: &Map, road_index: EdgeIndex, acceleration: f32, vehicle_max_speed: f32) -> f32 {
    let road = map
        .graph
        .edge_weight(road_index)
        .expect("get_minimal_time_travel_by_road called with invalid EdgeIndex (no corresponding road)");

    let max_speed = vehicle_max_speed.min(road.speed_limit);
    let acceleration_phase_length = 0.5 * max_speed * max_speed / acceleration;
    if road.length <= acceleration_phase_length {
        (2.0 * road.length / acceleration).sqrt()
    } else {
        max_speed / acceleration + (road.length - acceleration_phase_length) / max_speed
    }
}

pub fn get_minimal_co2_by_road(map: &Map, road_index: EdgeIndex) -> f32 {
    match map.graph.edge_weight(road_index) {
        Some(road) => {
            let cruise_speed = ((IDLE_POWER * DRIVE_TRAIN_EFFICIENCY)
                / (AIR_DENSITY * AERODYNAMIC_DRAG_COEFFICIENT * FRONT_AREA))
                .powf(1.0 / 3.0);
            let fuel_conversion_factor = STOICHIOMETRIC_CO2_FACTOR
                / (ENGINE_THERMAL_EFFICIENCY * LOWER_HEATING_VALUE_FOR_FUEL);
            let aerodynamic_drag_force = 0.5
                * AIR_DENSITY
                * AERODYNAMIC_DRAG_COEFFICIENT
                * FRONT_AREA
                * cruise_speed
                * cruise_speed;
            let rolling_resistance_force =
                VEHICLE_MASS * GRAVITY * ROLLING_RESISTANCE_COEFFICIENT;
            road.length
                * fuel_conversion_factor
                * (IDLE_POWER / cruise_speed
                    + (aerodynamic_drag_force + rolling_resistance_force) / DRIVE_TRAIN_EFFICIENCY)
        }
        None => 0.0,
    }
}

pub fn get_vehicle_min_time(vehicle: &Vehicle, map: &Map) -> f32 {
    let mut total_time: f32 = 0.0;

    if vehicle.path.len() < 2 {
        return total_time;
    }

    for i in 0..(vehicle.path.len() - 1) {
        let from = vehicle.path[i];
        let to = vehicle.path[i + 1];
        let edge = map
            .graph
            .find_edge(from, to)
            .ok_or("Edge not in map")
            .unwrap();

        total_time += get_minimal_time_travel_by_road(map, edge, vehicle.spec.max_acceleration, vehicle.spec.max_speed);
    }

    total_time
}

pub fn get_vehicle_min_co2(vehicle: &Vehicle, map: &Map) -> f32 {
    let mut total_co2: f32 = 0.0;

    if vehicle.path.len() < 2 {
        return total_co2;
    }

    for i in 0..(vehicle.path.len() - 1) {
        let from = vehicle.path[i];
        let to = vehicle.path[i + 1];
        let edge = map
            .graph
            .find_edge(from, to)
            .ok_or("Edge not in map")
            .unwrap();

        total_co2 += get_minimal_co2_by_road(map, edge);
    }

    total_co2
}

pub fn update_co2_emissions(vehicle: &mut Vehicle, time_step: f32) {
    let acceleration = (vehicle.velocity - vehicle.previous_velocity) / time_step;
    let tractive_force = (0.5
        * AIR_DENSITY
        * AERODYNAMIC_DRAG_COEFFICIENT
        * FRONT_AREA
        * vehicle.velocity
        * vehicle.velocity
        + VEHICLE_MASS * GRAVITY * ROLLING_RESISTANCE_COEFFICIENT
        + VEHICLE_MASS * acceleration)
        / DRIVE_TRAIN_EFFICIENCY;
    let current_emissions = (tractive_force * vehicle.velocity + IDLE_POWER)
        * STOICHIOMETRIC_CO2_FACTOR
        / (ENGINE_THERMAL_EFFICIENCY * LOWER_HEATING_VALUE_FOR_FUEL);
    vehicle.emitted_co2 += current_emissions * time_step;
}

#[derive(PartialEq)]
struct MinHeap(f64, usize);

impl Eq for MinHeap {}

impl Ord for MinHeap {
    fn cmp(&self, other: &Self) -> Ordering {
        other.0.partial_cmp(&self.0).unwrap_or(Ordering::Equal)
    }
}

impl PartialOrd for MinHeap {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn euclidean(p1: (f64, f64), p2: (f64, f64)) -> f64 {
    ((p1.0 - p2.0).powi(2) + (p1.1 - p2.1).powi(2)).sqrt()
}

fn mst_length(points: &[(f64, f64)]) -> f64 {
    let n = points.len();
    let mut visited = vec![false; n];
    let mut heap = BinaryHeap::new();
    heap.push(MinHeap(0.0, 0));
    let mut total = 0.0;

    while let Some(MinHeap(cost, u)) = heap.pop() {
        if visited[u] { continue; }
        visited[u] = true;
        total += cost;
        for v in 0..n {
            if !visited[v] {
                heap.push(MinHeap(euclidean(points[u], points[v]), v));
            }
        }
    }
    total
}

pub fn steiner_lower_bound(map: &Map) -> f64 {
    let points: Vec<(f64, f64)> = map
        .graph
        .node_indices()
        .filter(|&n| match map.graph[n].kind {
            IntersectionKind::Habitation | IntersectionKind::Workplace => true,
            _ => false,
        })
        .map(|n| {
            let node = &map.graph[n];
            (node.center_coordinates.x as f64, node.center_coordinates.y as f64)
        })
        .collect();

    if points.is_empty() {
        return 0.0;
    }

    (3.0_f64.sqrt() / 2.0) * mst_length(&points)
}

pub fn compute_score(vehicles: &[Vehicle], config: &SimulationConfig) -> f32 {
    let nb_arrived = vehicles.iter().filter(|v| matches!(v.state, VehicleState::Arrived)).count();
    let success_rate = if vehicles.is_empty() { 0.0 } else { nb_arrived as f32 / vehicles.len() as f32 };

    let total_trip_time: f32 = vehicles
        .iter()
        .filter(|v| matches!(v.state, VehicleState::Arrived))
        .filter_map(|v| v.arrived_at.map(|a| a - v.trip.departure_time))
        .sum();
    let total_ref_trip_time: f32 = vehicles
        .iter()
        .filter(|v| matches!(v.state, VehicleState::Arrived))
        .map(|v| get_vehicle_min_time(v, &config.map))
        .sum();

    let total_emitted_co2: f32 = vehicles
        .iter()
        .filter(|v| matches!(v.state, VehicleState::Arrived))
        .map(|v| v.emitted_co2)
        .sum();
    let total_ref_emitted_co2: f32 = vehicles
        .iter()
        .filter(|v| matches!(v.state, VehicleState::Arrived))
        .map(|v| get_vehicle_min_co2(v, &config.map))
        .sum();

    let best_network_length = steiner_lower_bound(&config.map);
    let mut seen_ids: HashSet<u32> = HashSet::new();
    let network_length: f32 = config
        .map
        .graph
        .edge_references()
        .filter_map(|er| {
            let road = er.weight();
            if seen_ids.insert(road.id) {
                Some(road.length)
            } else {
                None
            }
        })
        .sum();

    let time_term = if total_trip_time > 0.0 {
        total_ref_trip_time / total_trip_time
    } else {
        0.0
    };

    let pollution_term = if total_emitted_co2 > 0.0 {
        total_ref_emitted_co2 / total_emitted_co2
    } else {
        0.0
    };

    TIME_WEIGHT * time_term
        + SUCCESS_WEIGHT * success_rate
        + POLLUTION_WEIGHT * pollution_term
        + INFRASTRUCTURE_WEIGHT * (best_network_length as f32 / network_length)
}
