//! The traffic simulation engine.
//!
//! Each `step()` advances the world by `config.time_step`:
//! 1. departures: waiting vehicles enter their first road once there is room;
//! 2. signals: traffic light phases advance;
//! 3. planning: every vehicle lists the junctions ahead (its drive plan) and estimates
//!    when it will reach and clear each of them;
//! 4. lane changes;
//! 5. registration: drive plans are published on the links so that foes can see them;
//! 6. movement: every vehicle decides which links it may cross, follows its leader (IDM)
//!    and moves, possibly across several lanes.
//!
//! See `docs/intersection_algorithm.md` for the right-of-way model.

mod lane_change;
mod lanes;
mod movement;
pub(crate) mod network;
mod planning;
mod right_of_way;
mod signals;
mod trips;

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap};

use rustc_hash::FxHashMap;

use ordered_float::OrderedFloat;
use rand::rngs::StdRng;
use rand::SeedableRng;

use crate::scoring::{self, Score};
use crate::simulation::commute::CommutePlan;
use crate::simulation::config::SimulationConfig;
use crate::simulation::vehicle::{LaneId, Vehicle, VehicleState};

use network::Network;
use right_of_way::{Approach, QueueRank};
use signals::Signals;

pub use right_of_way::{time_windows_conflict, ApproachData, Blocker};

pub trait Simulation {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self;
    fn run(&mut self);
    fn step(&mut self);
    fn get_score(&self) -> Score;
}

#[derive(Clone)]
pub struct BusLine {
    pub id: u64,
    pub name: String,
    pub stop_node_ids: Vec<u32>,
    pub vehicle_id: u64,
}

#[derive(Clone)]
pub struct SimulationEngine {
    pub config: SimulationConfig,
    pub vehicles: Vec<Vehicle>,
    pub current_time: f32,
    /// Vehicles on each lane, sorted by position (back -> front).
    pub vehicles_by_lane: FxHashMap<LaneId, Vec<usize>>,
    pub all_vehicles_arrived: bool,
    pub bus_lines: Vec<BusLine>,
    pub next_bus_line_id: u64,
    pub next_vehicle_id: u64,
    /// Roads with a green light, per traffic light controller.
    pub controller_green_road_ids: HashMap<u32, Vec<u32>>,
    pub commute_plans: HashMap<u64, CommutePlan>,

    network: Network,
    signals: Signals,
    /// Index of each vehicle in its lane's list in `vehicles_by_lane`.
    lane_slots: Vec<Option<usize>>,
    /// Vehicles approaching each link, indexed by link id.
    approaches: Vec<Vec<Approach>>,
    /// Queue precedence of each vehicle at its next junction, refreshed every step.
    queue_ranks: Vec<Option<QueueRank>>,
    /// Space on junction exits already promised to vehicles inside the junction.
    exit_claims: FxHashMap<LaneId, f32>,
    /// Last vehicle that left each lane into a junction, used to keep following it.
    last_into_junction: FxHashMap<LaneId, usize>,
    departure_queue: BinaryHeap<Reverse<(OrderedFloat<f32>, usize)>>,
    vehicle_indices_by_id: HashMap<u64, usize>,
    rng: StdRng,
}

impl Simulation for SimulationEngine {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self {
        Self::new_with_commutes(config, vehicles, Vec::new())
    }

    fn run(&mut self) {
        for vehicle in &mut self.vehicles {
            vehicle.update_path(&self.config.map);
        }

        while self.current_time < self.config.end_time {
            self.step();
            self.current_time += self.config.time_step;
        }
    }

    fn get_score(&self) -> Score {
        scoring::compute_score(&self.vehicles, &self.config)
    }

    fn step(&mut self) {
        let dt = self.config.time_step;
        for vehicle in &mut self.vehicles {
            vehicle.previous_velocity = vehicle.velocity;
            vehicle.lane_change_cooldown = (vehicle.lane_change_cooldown - dt).max(0.0);
        }

        self.handle_departures();
        self.signals.advance(&self.config.map, &self.network, dt);
        self.plan_movements();
        self.attempt_lane_changes();
        self.register_approaches();
        self.execute_movements();

        for vehicle in &mut self.vehicles {
            if vehicle.state == VehicleState::OnRoad {
                scoring::update_co2_emissions(vehicle, dt);
            }
        }
        self.controller_green_road_ids.clone_from(&self.signals.green_road_ids);
    }
}

impl SimulationEngine {
    pub fn new_with_commutes(
        config: SimulationConfig,
        vehicles: Vec<Vehicle>,
        commute_plans: Vec<CommutePlan>,
    ) -> Self {
        let vehicle_indices_by_id = vehicles
            .iter()
            .enumerate()
            .map(|(index, vehicle)| (vehicle.id, index))
            .collect();

        let mut engine = Self {
            current_time: config.start_time,
            next_vehicle_id: vehicles.len() as u64,
            config,
            vehicles,
            vehicles_by_lane: FxHashMap::default(),
            all_vehicles_arrived: false,
            bus_lines: Vec::new(),
            next_bus_line_id: 0,
            controller_green_road_ids: HashMap::new(),
            commute_plans: commute_plans.into_iter().map(|plan| (plan.id, plan)).collect(),
            network: Network::default(),
            signals: Signals::default(),
            lane_slots: Vec::new(),
            approaches: Vec::new(),
            queue_ranks: Vec::new(),
            exit_claims: FxHashMap::default(),
            last_into_junction: FxHashMap::default(),
            departure_queue: BinaryHeap::new(),
            vehicle_indices_by_id,
            rng: StdRng::from_os_rng(),
        };

        engine.rebuild_runtime_caches();
        engine
    }

    /// Makes the engine's random decisions (navigation mistakes) reproducible.
    pub fn set_seed(&mut self, seed: u64) {
        self.rng = StdRng::seed_from_u64(seed);
    }

    /// Re-derives every cache from the map. Must be called after editing the map.
    pub fn rebuild_runtime_caches(&mut self) {
        self.network = Network::build(&self.config.map);
        self.signals = Signals::build(&self.config.map, &self.network, &self.signals);
        self.controller_green_road_ids.clone_from(&self.signals.green_road_ids);

        self.approaches = vec![Vec::new(); self.network.link_id_bound()];
        for vehicle in &mut self.vehicles {
            vehicle.registered_link_ids.clear();
        }
        self.queue_ranks = vec![None; self.vehicles.len()];
        self.lane_slots = vec![None; self.vehicles.len()];
        self.rebuild_lane_slots();
        self.rebuild_exit_claims();
        self.last_into_junction.clear();

        self.departure_queue.clear();
        for vidx in 0..self.vehicles.len() {
            self.enqueue_departure(vidx);
        }
        self.refresh_all_arrived();
    }

    fn rebuild_exit_claims(&mut self) {
        self.exit_claims.clear();
        for (lane, list) in &self.vehicles_by_lane {
            let Some(info) = self.network.link_of_internal_lane(*lane) else {
                continue;
            };
            let claimed: f32 = list
                .iter()
                .map(|&vidx| self.vehicles[vidx].spec.length + self.config.minimum_gap)
                .sum();
            *self.exit_claims.entry(info.out_lane).or_default() += claimed;
        }
    }

    /// Number of junction approaches currently registered by `vidx`.
    #[cfg(test)]
    pub(crate) fn approach_count(&self, vidx: usize) -> usize {
        self.approaches.iter().flatten().filter(|a| a.vehicle == vidx).count()
    }

    /// Lanes holding vehicles, in a deterministic order.
    fn occupied_lanes(&self) -> Vec<LaneId> {
        let mut lanes: Vec<LaneId> = self.vehicles_by_lane.keys().copied().collect();
        lanes.sort_unstable();
        lanes
    }
}

#[cfg(test)]
pub(crate) fn lane_insert_sorted(
    by_lane: &mut HashMap<LaneId, Vec<usize>>,
    vehicles: &[Vehicle],
    lane: LaneId,
    vehicle_idx: usize,
) {
    let list = by_lane.entry(lane).or_default();
    let insert_at = list.partition_point(|&i| {
        vehicles[i].position_on_lane < vehicles[vehicle_idx].position_on_lane
    });
    list.insert(insert_at, vehicle_idx);
}
