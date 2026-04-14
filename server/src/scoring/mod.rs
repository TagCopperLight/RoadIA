use crate::map::model::Map;
use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{Vehicle, VehicleState, VehicleType};
use petgraph::graph::EdgeIndex;
use crate::map::intersection::IntersectionKind;
use std::collections::{BinaryHeap, HashSet};
use std::cmp::Ordering;

// Vehicle Physics Constants
const VEHICLE_MASS: f32 = 1680.0; // kg - typical passenger car
const AIR_DENSITY: f32 = 1.225; // kg/m³
const GRAVITY: f32 = 9.81; // m/s²

// Vehicle-specific constants by motorization
// ========================================
// Coefficients de traînée aérodynamique (adjusted per vehicle type)
const DRAG_COEFFICIENT_HYBRID: f32 = 0.30;      // Optimisé hybride
const DRAG_COEFFICIENT_ELECTRIC: f32 = 0.28;    // Aérodynamique améliorée (Tesla style)
const DRAG_COEFFICIENT_THERMAL: f32 = 0.32;     // Essence standard
const DRAG_COEFFICIENT_DIESEL: f32 = 0.31;      // Diesel SUV-like

// Aire frontale (m²)
const FRONT_AREA: f32 = 2.0;

// Coefficient de résistance au roulement
const ROLLING_RESISTANCE: f32 = 0.01;

// Efficacité moteur (thermique → mécanique). EV très différent!
// Thermal efficiency (combustion engine conversion efficiency)
const EFFICIENCY_THERMAL: f32 = 0.32; // ~32% pour essence
const EFFICIENCY_DIESEL: f32 = 0.40;  // ~40% pour diesel (meilleur)
const EFFICIENCY_HYBRID: f32 = 0.75;  // ~75% efficacité moyenne hybrid (electric + thermal)
const EFFICIENCY_ELECTRIC: f32 = 0.90; // ~90% efficacité moteur électrique + batterie

// Puissance au ralenti (W) - Idle consumption
const IDLE_POWER_THERMAL: f32 = 2500.0;
const IDLE_POWER_DIESEL: f32 = 2800.0;
const IDLE_POWER_HYBRID: f32 = 500.0;   // Moteur thermique éteint souvent
const IDLE_POWER_ELECTRIC: f32 = 50.0;  // Consommation auxiliaire (clim, électronique)

// Conversion factors: fuel energy → CO₂
// WLTP: 1L essence ≈ 2.31 kg CO₂; 1L diesel ≈ 2.68 kg CO₂; 1kWh électrique ≈ 0.05-0.15 kg CO₂ (grid avg)
const CO2_PER_LITER_THERMAL: f32 = 2.31;  // kg CO₂ per liter gasoline (WLTP)
const CO2_PER_LITER_DIESEL: f32 = 2.68;   // kg CO₂ per liter diesel (WLTP)
const CO2_PER_KWH_ELECTRIC: f32 = 0.10;   // kg CO₂ per kWh (EU grid average ~100g/kWh)

// Fuel density & energy content
const ENERGY_CONTENT_THERMAL: f32 = 43.2; // MJ/kg pour essence
const ENERGY_CONTENT_DIESEL: f32 = 45.5;  // MJ/kg pour diesel
const ENERGY_CONTENT_BATTERY: f32 = 0.0036; // MJ/Wh ≈ 3.6 MJ/kWh

// Score weights (must sum to 1.0)
const TIME_WEIGHT: f32 = 0.4;
const SUCCESS_WEIGHT: f32 = 0.2;
const POLLUTION_WEIGHT: f32 = 0.2;
const INFRASTRUCTURE_WEIGHT: f32 = 0.2;

// ============= Helper Functions for Vehicle Type Constants =============

fn get_drag_coefficient(vehicle_type: VehicleType) -> f32 {
    match vehicle_type {
        VehicleType::Electrique => DRAG_COEFFICIENT_ELECTRIC,
        VehicleType::Hybride => DRAG_COEFFICIENT_HYBRID,
        VehicleType::Essence => DRAG_COEFFICIENT_THERMAL,
        VehicleType::Diesel => DRAG_COEFFICIENT_DIESEL,
    }
}

fn get_efficiency(vehicle_type: VehicleType) -> f32 {
    match vehicle_type {
        VehicleType::Electrique => EFFICIENCY_ELECTRIC,
        VehicleType::Hybride => EFFICIENCY_HYBRID,
        VehicleType::Essence => EFFICIENCY_THERMAL,
        VehicleType::Diesel => EFFICIENCY_DIESEL,
    }
}

fn get_idle_power(vehicle_type: VehicleType) -> f32 {
    match vehicle_type {
        VehicleType::Electrique => IDLE_POWER_ELECTRIC,
        VehicleType::Hybride => IDLE_POWER_HYBRID,
        VehicleType::Essence => IDLE_POWER_THERMAL,
        VehicleType::Diesel => IDLE_POWER_DIESEL,
    }
}

fn get_co2_conversion_factor(vehicle_type: VehicleType) -> (f32, f32) {
    // Returns (co2_per_energy_unit, energy_content_per_fuel_unit)
    // For consistent calculation: co2_emitted = energy_consumed * factor
    match vehicle_type {
        VehicleType::Electrique => {
            // CO₂ from grid electricity
            (CO2_PER_KWH_ELECTRIC / 3.6, ENERGY_CONTENT_BATTERY) // Convert kWh to MJ and CO₂ back
        }
        VehicleType::Hybride => {
            // Average of thermal + electric (50/50 assumption for mixed driving)
            let thermal_factor = CO2_PER_LITER_THERMAL / ENERGY_CONTENT_THERMAL;
            let electric_factor = CO2_PER_KWH_ELECTRIC / (ENERGY_CONTENT_BATTERY * 3600.0);
            ((thermal_factor + electric_factor) / 2.0, ENERGY_CONTENT_THERMAL) // Simplified
        }
        VehicleType::Essence => {
            (CO2_PER_LITER_THERMAL / ENERGY_CONTENT_THERMAL, ENERGY_CONTENT_THERMAL)
        }
        VehicleType::Diesel => {
            (CO2_PER_LITER_DIESEL / ENERGY_CONTENT_DIESEL, ENERGY_CONTENT_DIESEL)
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

pub fn get_minimal_co2_by_road(map: &Map, road_index: EdgeIndex, vehicle_type: VehicleType) -> f32 {
    match map.graph.edge_weight(road_index) {
        Some(road) => {
            // Calculate minimal CO2 for optimal (eco) driving on this road
            // Assume: constant speed = road.speed_limit, no acceleration after initial phase
            
            let drag_coeff = get_drag_coefficient(vehicle_type);
            let efficiency = get_efficiency(vehicle_type);
            let idle_power = get_idle_power(vehicle_type);
            let (co2_factor, _) = get_co2_conversion_factor(vehicle_type);
            
            // Estimate cruise speed (realistic: 80% of speed limit for efficiency)
            let cruise_speed = (road.speed_limit * 0.8).min(130.0 / 3.6); // Max ~130 km/h = 36 m/s
            
            if cruise_speed < 0.1 {
                return 0.0;
            }
            
            // Power needed at cruise:
            // P_aerodynamic = 0.5 * ρ * Cd * A * v³
            let aerodynamic_force = 0.5 * AIR_DENSITY * drag_coeff * FRONT_AREA * cruise_speed * cruise_speed;
            let rolling_resistance_force = VEHICLE_MASS * GRAVITY * ROLLING_RESISTANCE;
            
            // Total power at cruise = (F_aero + F_rolling) * v + P_idle
            let cruise_power_w = (aerodynamic_force + rolling_resistance_force) * cruise_speed + idle_power;
            
            // Energy consumed over the road distance
            let time_hours = road.length / (cruise_speed * 3.6); // Convert m/s to km/h for time calc
            let energy_mj = cruise_power_w * time_hours * 3.6 / 1000.0; // Convert W·hour to MJ
            
            // Account for drive-train efficiency
            let energy_input_mj = energy_mj / efficiency;
            
            // CO₂ emitted (co2_factor is in g/MJ, result is in grams)
            (energy_input_mj * co2_factor).max(0.0)
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

        total_co2 += get_minimal_co2_by_road(map, edge, vehicle.spec.vehicle_type);
    }

    total_co2
}

pub fn update_co2_emissions(vehicle: &mut Vehicle, time_step: f32) {
    // Calculate real-world CO₂ emissions based on instantaneous power consumption
    // adapted to vehicle type
    
    let vehicle_type = vehicle.spec.vehicle_type;
    let velocity = vehicle.velocity;
    
    // Skip if stationary
    if velocity < 0.1 && (vehicle.velocity - vehicle.previous_velocity).abs() < 0.1 {
        // Even at standstill, idle power consumes energy
        let idle_power = get_idle_power(vehicle_type);
        let efficiency = get_efficiency(vehicle_type);
        let (co2_factor, _) = get_co2_conversion_factor(vehicle_type);
        
        let energy_mj = idle_power * time_step / 3_600_000.0; // W·s to MJ
        let co2_grams = (energy_mj / efficiency) * co2_factor; // Result already in grams (MJ * g/MJ = grams)
        vehicle.emitted_co2 += co2_grams;
        return;
    }
    
    // Physics-based power calculation
    let drag_coeff = get_drag_coefficient(vehicle_type);
    let efficiency = get_efficiency(vehicle_type);
    let idle_power = get_idle_power(vehicle_type);
    let (co2_factor, _) = get_co2_conversion_factor(vehicle_type);
    
    // Calculate acceleration
    let acceleration = (velocity - vehicle.previous_velocity) / time_step;
    
    // Calculate forces
    // Aerodynamic drag: F_drag = 0.5 * ρ * Cd * A * v²
    let aerodynamic_drag = 0.5 * AIR_DENSITY * drag_coeff * FRONT_AREA * velocity * velocity;
    
    // Rolling resistance: F_rolling = μ * m * g
    let rolling_resistance = VEHICLE_MASS * GRAVITY * ROLLING_RESISTANCE;
    
    // Tractive force needed: F_traction = m*a + F_drag + F_rolling
    let tractive_force = VEHICLE_MASS * acceleration + aerodynamic_drag + rolling_resistance;
    
    // Power = Force * velocity + idle power
    // (idle power when accelerating slowly or in congestion)
    let motive_power = (tractive_force * velocity).max(0.0) + idle_power;
    
    // Energy consumed (convert from W to MJ over time_step)
    let energy_consumed_joules = motive_power * time_step;
    let energy_consumed_mj = energy_consumed_joules / 1_000_000.0;
    
    // Account for efficiency losses (power drawn from fuel/battery)
    let energy_input_mj = energy_consumed_mj / efficiency;
    
    // Convert to CO₂ (already accounts for fuel type)
    // co2_factor is in g/MJ, so result is directly in grams
    let co2_grams = energy_input_mj * co2_factor;
    
    vehicle.emitted_co2 += co2_grams.max(0.0);
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

pub struct Score {
    pub score: f32,
    pub total_trip_time: f32,
    pub total_emitted_co2: f32,
    pub network_length: f32,
    pub total_distance_traveled: f32,
    pub success_rate: f32,
}

pub fn compute_score(vehicles: &[Vehicle], config: &SimulationConfig) -> Score {
    let nb_arrived = vehicles.iter().filter(|v| matches!(v.state, VehicleState::Arrived)).count();
    let success_rate = if vehicles.is_empty() { 0.0 } else { nb_arrived as f32 / vehicles.len() as f32 };

    let total_trip_time: f32 = vehicles
        .iter()
        .filter(|v| matches!(v.state, VehicleState::Arrived))
        .filter_map(|v| v.arrived_at.map(|a| a - v.trip.departure_time))
        .fold(0.0_f32, f32::max);
    let sum_trip_time: f32 = vehicles
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

    let time_term = if sum_trip_time > 0.0 {
        total_ref_trip_time / sum_trip_time
    } else {
        0.0
    };

    let pollution_term = if total_emitted_co2 > 0.0 {
        total_ref_emitted_co2 / total_emitted_co2
    } else {
        0.0
    };

    let score = TIME_WEIGHT * time_term
        + SUCCESS_WEIGHT * success_rate
        + POLLUTION_WEIGHT * pollution_term
        + INFRASTRUCTURE_WEIGHT * (best_network_length as f32 / network_length);

    let total_distance_traveled: f32 = vehicles
        .iter()
        .filter(|v| matches!(v.state, VehicleState::Arrived))
        .map(|v| v.distance_traveled)
        .sum();

    Score {
        score,
        total_trip_time,
        total_emitted_co2,
        network_length,
        total_distance_traveled,
        success_rate,
    }
}
