use crate::map::model::Map;
use crate::map::road::{Road};
use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{Vehicle, VehicleSpec, VehicleState};
use petgraph::graph::{EdgeIndex, NodeIndex, Graph};
use petgraph::algo::{astar, bellman_ford};
use petgraph::visit::EdgeRef;
use crate::map::intersection::IntersectionKind;
use std::collections::{HashMap, HashSet};

use std::collections::BinaryHeap;
use std::cmp::Ordering;

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

pub fn get_minimal_co2_by_road(map: &Map, road_index : EdgeIndex, vehicle_spec : VehicleSpec, simulation_config : &SimulationConfig) -> f32 {
        match map.graph.edge_weight(road_index){
            Some(road) => {
                let cruise_speed = ((vehicle_spec.idle_power * vehicle_spec.drive_train_efficiency) / (simulation_config.air_density * vehicle_spec.aerodynamic_drag_coefficient * vehicle_spec.front_area)).powf(1.0/3.0);
                let fuel_conversion_factor = vehicle_spec.stoichiometric_co2_factor / (vehicle_spec.engine_thermal_efficiency * vehicle_spec.lower_heating_value_for_fuel);
                let aerodynamic_drag_force = 0.5*simulation_config.air_density * vehicle_spec.aerodynamic_drag_coefficient * vehicle_spec.front_area * cruise_speed * cruise_speed;
                let rolling_resistance_force = vehicle_spec.mass * simulation_config.gravity_coefficient * vehicle_spec.rolling_resistance_coefficient;
                road.length * fuel_conversion_factor * (vehicle_spec.idle_power / cruise_speed + (aerodynamic_drag_force + rolling_resistance_force) / vehicle_spec.drive_train_efficiency)
            },
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

pub fn get_vehicle_min_co2(vehicle: &Vehicle, sim_config: &SimulationConfig) -> f32 {
    let mut total_co2: f32 = 0.0;

    if vehicle.path.len() < 2 {
        return total_co2;
    }

    for i in 0..(vehicle.path.len() - 1) {
        let from = vehicle.path[i];
        let to = vehicle.path[i + 1];
        let edge = sim_config
            .map
            .graph
            .find_edge(from, to)
            .ok_or("Edge not in map")
            .unwrap();

        total_co2 += get_minimal_co2_by_road(&sim_config.map, edge, vehicle.spec, sim_config);
    }

    total_co2
}

pub fn update_co2_emissions(vehicle : &mut Vehicle, config: &SimulationConfig) {
        let acceleration = (vehicle.velocity - vehicle.previous_velocity)/config.time_step;
        let tractive_force = (0.5*config.air_density*vehicle.spec.aerodynamic_drag_coefficient*vehicle.spec.front_area*vehicle.velocity*vehicle.velocity + vehicle.spec.mass * config.gravity_coefficient * vehicle.spec.rolling_resistance_coefficient + vehicle.spec.mass * acceleration) / vehicle.spec.drive_train_efficiency;
        let current_emissions = (tractive_force * vehicle.velocity + vehicle.spec.idle_power) * vehicle.spec.stoichiometric_co2_factor / (vehicle.spec.engine_thermal_efficiency * vehicle.spec.lower_heating_value_for_fuel);
        vehicle.emitted_co2 += current_emissions * config.time_step;
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
            (node.x as f64, node.y as f64)
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
        .filter_map(|v| v.arrived_at.map(|a| a - v.trip.departure_time as f32))
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
        .map(|v| get_vehicle_min_co2(v, config))
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

    //println!("Best estimated total length {} | Actual network length {}", best_network_length, network_length);

    //println!("Empirical / Theoretical Co2 {} / {}", total_emitted_co2, total_ref_emitted_co2);

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

    config.time_weight * time_term + config.success_weight * success_rate + config.pollution_weight * pollution_term + config.infrastructure_weight * (best_network_length as f32 / network_length as f32)
}
