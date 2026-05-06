use crate::map::model::Map;
use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{Vehicle, VehicleState, VehicleType, VehicleKind};
use petgraph::graph::EdgeIndex;
use crate::map::intersection::IntersectionKind;
use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;

const AIR_DENSITY: f32 = 1.225;  // kg/m³
const GRAVITY: f32 = 9.81;       // m/s²
const DRIVE_TRAIN_EFFICIENCY: f32 = 0.90;

const FRANCE_GRID_CO2_PER_J: f32 = 50e-3 / 3_600_000.0; // kg CO2 per J of electricity

struct KindParams {
    mass: f32,              // kg
    drag_coeff: f32,        // dimensionless
    front_area: f32,        // m²
    rolling_resistance: f32, // dimensionless
    idle_combustion_w: f32, // W of fuel power consumed at standstill (combustion)
    idle_electric_w: f32,   // W of electrical power consumed at standstill (accessories)
}

fn kind_params(kind: VehicleKind) -> KindParams {
    match kind {
        VehicleKind::Car => KindParams {
            mass: 1_680.0,
            drag_coeff: 0.30,
            front_area: 2.0,
            rolling_resistance: 0.010,
            idle_combustion_w: 2_500.0,
            idle_electric_w: 400.0,
        },
        VehicleKind::Bus => KindParams {
            mass: 13_000.0,
            drag_coeff: 0.60,
            front_area: 7.0,
            rolling_resistance: 0.008,
            idle_combustion_w: 8_000.0,
            idle_electric_w: 1_500.0,
        },
    }
}

// Returns (co2_per_j_mech, idle_co2_per_s):
//   co2_per_j_mech: kg CO2 emitted per joule of mechanical work at the wheels
//   idle_co2_per_s: kg CO2 emitted per second while stationary
fn emission_params(motorization: VehicleType, kind: VehicleKind) -> (f32, f32) {
    let k = kind_params(kind);
    match motorization {
        VehicleType::Essence => {
            // Petrol: stoichiometric CO2 3.16 kg/kg_fuel, LHV 43.2 MJ/kg, thermal eff 35%
            let co2_per_j_mech = 3.16 / (0.35 * DRIVE_TRAIN_EFFICIENCY * 43_200_000.0);
            let idle_co2_per_s = k.idle_combustion_w * 3.16 / (0.35 * 43_200_000.0);
            (co2_per_j_mech, idle_co2_per_s)
        }
        VehicleType::Diesel => {
            // Diesel: stoichiometric CO2 3.17 kg/kg_fuel, LHV 43.1 MJ/kg, thermal eff 42%
            // Higher efficiency than petrol yields lower CO2 per unit of work
            let co2_per_j_mech = 3.17 / (0.42 * DRIVE_TRAIN_EFFICIENCY * 43_100_000.0);
            let idle_co2_per_s = k.idle_combustion_w * 3.17 / (0.42 * 43_100_000.0);
            (co2_per_j_mech, idle_co2_per_s)
        }
        VehicleType::Electrique => {
            // Electric: emissions come from the power grid, not combustion
            // Motor efficiency 92%; drivetrain efficiency 90%
            let co2_per_j_mech = FRANCE_GRID_CO2_PER_J / (0.92 * DRIVE_TRAIN_EFFICIENCY);
            let idle_co2_per_s = k.idle_electric_w * FRANCE_GRID_CO2_PER_J;
            (co2_per_j_mech, idle_co2_per_s)
        }
        VehicleType::Hybride => {
            // Plug-in hybrid: ~50% of driving in electric mode, ~50% in petrol mode
            // (urban driving skews more electric; highway more combustion)
            let elec_co2_per_j = FRANCE_GRID_CO2_PER_J / (0.92 * DRIVE_TRAIN_EFFICIENCY);
            let comb_co2_per_j = 3.16 / (0.38 * DRIVE_TRAIN_EFFICIENCY * 43_200_000.0);
            let co2_per_j_mech = 0.50 * elec_co2_per_j + 0.50 * comb_co2_per_j;
            let idle_co2_per_s = 0.50 * k.idle_electric_w * FRANCE_GRID_CO2_PER_J
                + 0.50 * k.idle_combustion_w * 3.16 / (0.38 * 43_200_000.0);
            (co2_per_j_mech, idle_co2_per_s)
        }
    }
}

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

pub fn get_minimal_co2_by_road(map: &Map, road_index: EdgeIndex, motorization: VehicleType, kind: VehicleKind) -> f32 {
    match map.graph.edge_weight(road_index) {
        Some(road) => {
            let k = kind_params(kind);
            let (co2_per_j_mech, idle_co2_per_s) = emission_params(motorization, kind);

            let drag_k = AIR_DENSITY * k.drag_coeff * k.front_area; // F_aero = 0.5 * drag_k * v²
            let roll_force = k.mass * GRAVITY * k.rolling_resistance;

            // Optimal cruise speed minimises CO2/distance:
            //   CO2/m = 0.5*drag_k*v² * co2_per_j + roll_force * co2_per_j + idle_co2/v
            //   d/dv = drag_k * v * co2_per_j - idle_co2/v² = 0
            //   v³ = idle_co2_per_s / (drag_k * co2_per_j_mech)
            let cruise_speed = (idle_co2_per_s / (drag_k * co2_per_j_mech))
                .powf(1.0 / 3.0)
                .min(road.speed_limit);

            let f_aero = 0.5 * drag_k * cruise_speed * cruise_speed;
            let co2_per_m = (f_aero + roll_force) * co2_per_j_mech + idle_co2_per_s / cruise_speed;

            road.length * co2_per_m
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

        total_co2 += get_minimal_co2_by_road(map, edge, vehicle.motorization, vehicle.spec.kind);
    }

    total_co2
}

pub fn update_co2_emissions(vehicle: &mut Vehicle, time_step: f32) {
    let k = kind_params(vehicle.spec.kind);
    let (co2_per_j_mech, idle_co2_per_s) = emission_params(vehicle.motorization, vehicle.spec.kind);

    let acceleration = (vehicle.velocity - vehicle.previous_velocity) / time_step;

    // Net force required at the wheels (Newton's 2nd law)
    let wheel_force = 0.5 * AIR_DENSITY * k.drag_coeff * k.front_area * vehicle.velocity * vehicle.velocity
        + k.mass * GRAVITY * k.rolling_resistance
        + k.mass * acceleration;

    // Clamp to zero: no regenerative braking model; engine braking emits idle only
    let p_mech = (wheel_force * vehicle.velocity).max(0.0);

    let co2_per_s = p_mech * co2_per_j_mech + idle_co2_per_s;
    vehicle.emitted_co2 += co2_per_s * time_step;
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
        .filter(|&n| matches!(map.graph[n].kind, IntersectionKind::Habitation | IntersectionKind::Workplace))
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

#[derive(Clone, Copy, Default)]
pub struct Score {
    pub score: f32,
    pub total_trip_time: f32,
    pub ref_total_trip_time: f32,
    pub total_emitted_co2: f32,
    pub ref_total_emitted_co2: f32,
    pub network_length: f32,
    pub ref_network_length: f32,
    pub success_rate: f32,
}

pub fn compute_score(vehicles: &[Vehicle], config: &SimulationConfig) -> Score {
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

    let best_network_length = steiner_lower_bound(&config.map) as f32;
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

    let w = &config.score_weights;
    // le score possède une valeur entre 0 et 100
    let score = (w.time * time_term
        + w.success * success_rate
        + w.pollution * pollution_term
        + w.infrastructure * (best_network_length / network_length)) * 100f32;

    Score {
        score,
        total_trip_time,
        ref_total_trip_time: total_ref_trip_time,
        total_emitted_co2,
        ref_total_emitted_co2: total_ref_emitted_co2,
        network_length,
        ref_network_length: best_network_length,
        success_rate,
    }
}
