use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

use ordered_float::OrderedFloat;
use crate::scoring;
use crate::simulation::commute::{CommutePlan, CommutePlanState};
use crate::simulation::config::{
    SimulationConfig, IMPATIENCE_RATE, LOOK_AHEAD, MIN_CREEP_SPEED, STOP_DWELL_TIME,
    LC2013_COOLDOWN, LC2013_SPEED_GAIN_THRESHOLD, LC2013_SAFE_GAP_FRONT,
    LC2013_SAFE_GAP_REAR, LC2013_MAX_DECEL_FOLLOWER,
};
use crate::map::intersection::{ApproachData, LinkState, is_link_open};
use crate::scoring::Score;
use crate::simulation::kinematics;
use crate::simulation::vehicle::{DrivePlanEntry, LaneId, TripRequest, Vehicle, VehicleKind, VehicleSpec, VehicleState, VehicleType};
use petgraph::graph::{EdgeIndex, NodeIndex};

pub trait Simulation {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self;
    fn run(&mut self);
    fn step(&mut self);
    fn get_score(&self) -> Score;
}

#[derive(Clone)]
struct PendingTransfer {
    vehicle_idx: usize,
    from_lane: LaneId,
    to_lane: Option<LaneId>,
}

#[derive(Clone)]
struct TrafficLightRuntimeState {
    phase_index: usize,
    time_in_phase: f32,
}

#[derive(Clone)]
pub struct BusLine {
    pub id: u64,
    pub name: String,
    pub stop_node_ids: Vec<u32>,
    pub vehicle_id: u64,
}

#[derive(Clone)]
struct InternalLaneRuntime {
    length: f32,
    speed_limit: f32,
    to_lane_id: u32,
}

#[derive(Clone)]
pub struct SimulationEngine {
    pub config: SimulationConfig,
    pub vehicles: Vec<Vehicle>,
    pub current_time: f32,
    pub vehicles_by_lane: HashMap<LaneId, Vec<usize>>, // Sorted by position_on_lane (back -> front).
    vehicle_lane_positions: Vec<Option<usize>>,
    pub link_states: HashMap<u32, LinkState>,
    pub all_vehicles_arrived: bool,
    pub green_links: HashSet<u32>,
    pub bus_lines: Vec<BusLine>,
    pub next_bus_line_id: u64,
    pub next_vehicle_id: u64,
    departure_queue: BinaryHeap<Reverse<(OrderedFloat<f32>, usize)>>,
    pending_transfers: Vec<PendingTransfer>,
    traffic_light_states: HashMap<u32, TrafficLightRuntimeState>,
    link_directory: HashMap<u32, crate::map::road::Link>,
    link_to_road_id: HashMap<u32, u32>,
    internal_lane_directory: HashMap<(u32, u32), InternalLaneRuntime>,
    controller_green_link_ids: HashMap<u32, HashSet<u32>>,
    pub controller_green_road_ids: HashMap<u32, Vec<u32>>,
    pub commute_plans: HashMap<u64, CommutePlan>,
    vehicle_indices_by_id: HashMap<u64, usize>,
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
            if vehicle.lane_change_cooldown > 0.0 {
                vehicle.lane_change_cooldown = (vehicle.lane_change_cooldown - dt).max(0.0);
            }
        }

        self.handle_departures();
        self.plan_movements();
        self.attempt_lane_changes();
        self.register_approaches();
        self.advance_traffic_lights();
        self.execute_movements();
        self.flush_transfers();

        for vehicle in &mut self.vehicles {
            if vehicle.state == VehicleState::OnRoad {
                scoring::update_co2_emissions(vehicle, dt);
            }
        }
    }
}

impl SimulationEngine {
    pub(crate) fn new_with_commutes(
        config: SimulationConfig,
        vehicles: Vec<Vehicle>,
        commute_plans: Vec<CommutePlan>,
    ) -> Self {
        let current_time = config.start_time;

        let commute_plans = commute_plans.into_iter().map(|plan| (plan.id, plan)).collect();
        let vehicle_indices_by_id = vehicles
            .iter()
            .enumerate()
            .map(|(index, vehicle)| (vehicle.id, index))
            .collect();
        let next_vehicle_id = vehicles.len() as u64;

        let mut engine = Self {
            config,
            vehicles,
            current_time,
            vehicles_by_lane: HashMap::new(),
            vehicle_lane_positions: Vec::new(),
            link_states: HashMap::new(),
            all_vehicles_arrived: false,
            green_links: HashSet::new(),
            bus_lines: Vec::new(),
            next_bus_line_id: 0,
            next_vehicle_id,
            departure_queue: BinaryHeap::new(),
            pending_transfers: Vec::new(),
            traffic_light_states: HashMap::new(),
            link_directory: HashMap::new(),
            link_to_road_id: HashMap::new(),
            internal_lane_directory: HashMap::new(),
            controller_green_link_ids: HashMap::new(),
            controller_green_road_ids: HashMap::new(),
            commute_plans,
            vehicle_indices_by_id,
        };

        engine.rebuild_runtime_caches();
        engine
    }

    pub(crate) fn rebuild_runtime_caches(&mut self) {
        let previous_traffic_light_states = self.traffic_light_states.clone();

        self.link_directory.clear();
        self.link_to_road_id.clear();
        self.internal_lane_directory.clear();
        self.traffic_light_states.clear();
        self.green_links.clear();
        self.controller_green_link_ids.clear();
        self.controller_green_road_ids.clear();
        self.link_states.clear();
        self.pending_transfers.clear();

        for edge in self.config.map.graph.edge_indices() {
            let road_id = self.config.map.graph[edge].id;
            for lane in &self.config.map.graph[edge].lanes {
                for link in &lane.links {
                    self.link_directory.insert(link.id, link.clone());
                    self.link_to_road_id.insert(link.id, road_id);
                }
            }
        }

        for node_idx in self.config.map.graph.node_indices() {
            let junction_id = self.config.map.graph[node_idx].id;
            for lane in &self.config.map.graph[node_idx].internal_lanes {
                self.internal_lane_directory.insert(
                    (junction_id, lane.id),
                    InternalLaneRuntime {
                        length: lane.length,
                        speed_limit: lane.speed_limit,
                        to_lane_id: lane.to_lane_id,
                    },
                );
            }
        }

        for &controller_id in self.config.map.traffic_lights.keys() {
            let state = if let Some(existing) = previous_traffic_light_states.get(&controller_id) {
                let controller = &self.config.map.traffic_lights[&controller_id];
                if existing.phase_index < controller.phases.len() {
                    existing.clone()
                } else {
                    TrafficLightRuntimeState {
                        phase_index: 0,
                        time_in_phase: 0.0,
                    }
                }
            } else {
                TrafficLightRuntimeState {
                    phase_index: 0,
                    time_in_phase: 0.0,
                }
            };
            self.traffic_light_states.insert(controller_id, state);
        }
        for (&controller_id, state) in &self.traffic_light_states {
            let (active_links, active_roads) = Self::traffic_light_cache_for_controller(
                &self.config.map,
                &self.link_to_road_id,
                controller_id,
                state,
            );
            self.green_links.extend(active_links.iter().copied());
            self.controller_green_link_ids
                .insert(controller_id, active_links);
            self.controller_green_road_ids
                .insert(controller_id, active_roads);
        }

        self.vehicle_lane_positions = vec![None; self.vehicles.len()];
        self.rebuild_lane_position_cache();
        self.seed_departure_queue();
        self.all_vehicles_arrived = self
            .vehicles
            .iter()
            .all(|vehicle| matches!(vehicle.state, VehicleState::Arrived));
    }
}

// Runtime caches and queues
impl SimulationEngine {
    fn traffic_light_cache_for_controller(
        map: &crate::map::model::Map,
        link_to_road_id: &HashMap<u32, u32>,
        controller_id: u32,
        state: &TrafficLightRuntimeState,
    ) -> (HashSet<u32>, Vec<u32>) {
        let mut active_link_ids = HashSet::new();
        let mut active_road_ids = HashSet::new();

        let controller = match map.traffic_lights.get(&controller_id) {
            Some(controller) => controller,
            None => return (active_link_ids, Vec::new()),
        };

        if controller.phases.is_empty() {
            return (active_link_ids, Vec::new());
        }

        let phase = &controller.phases[state.phase_index];
        if state.time_in_phase < phase.green_duration {
            for &link_id in &phase.green_link_ids {
                active_link_ids.insert(link_id);
                if let Some(&road_id) = link_to_road_id.get(&link_id) {
                    active_road_ids.insert(road_id);
                }
            }
        }

        let mut active_road_ids: Vec<u32> = active_road_ids.into_iter().collect();
        active_road_ids.sort_unstable();
        (active_link_ids, active_road_ids)
    }

    fn seed_departure_queue(&mut self) {
        self.departure_queue.clear();
        for vidx in 0..self.vehicles.len() {
            self.enqueue_departure(vidx);
        }
    }

    fn enqueue_departure(&mut self, vidx: usize) {
        let vehicle = &self.vehicles[vidx];
        if vehicle.state != VehicleState::WaitingToDepart {
            return;
        }
        let departure_time = OrderedFloat(vehicle.trip.departure_time);
        if departure_time.into_inner() >= f32::MAX {
            return;
        }
        self.departure_queue.push(Reverse((departure_time, vidx)));
    }

    fn rebuild_lane_position_cache(&mut self) {
        for slot in &mut self.vehicle_lane_positions {
            *slot = None;
        }

        for indices in self.vehicles_by_lane.values() {
            for (slot, &vehicle_idx) in indices.iter().enumerate() {
                if let Some(entry) = self.vehicle_lane_positions.get_mut(vehicle_idx) {
                    *entry = Some(slot);
                }
            }
        }
    }

    pub(crate) fn insert_vehicle_into_lane(&mut self, lane: LaneId, vehicle_idx: usize) {
        let insert_at = {
            let list = self.vehicles_by_lane.entry(lane).or_default();
            let insert_at = list.partition_point(|&i| {
                self.vehicles[i].position_on_lane < self.vehicles[vehicle_idx].position_on_lane
            });
            list.insert(insert_at, vehicle_idx);
            insert_at
        };

        if let Some(slot) = self.vehicle_lane_positions.get_mut(vehicle_idx) {
            *slot = Some(insert_at);
        }

        if let Some(list) = self.vehicles_by_lane.get(&lane) {
            for (slot, &other_idx) in list.iter().enumerate().skip(insert_at + 1) {
                if let Some(entry) = self.vehicle_lane_positions.get_mut(other_idx) {
                    *entry = Some(slot);
                }
            }
        }
    }

    pub(crate) fn remove_vehicle_from_lane(&mut self, lane: LaneId, vehicle_idx: usize) {
        let (removed_slot, remove_lane) = {
            let Some(list) = self.vehicles_by_lane.get_mut(&lane) else {
                if let Some(slot) = self.vehicle_lane_positions.get_mut(vehicle_idx) {
                    *slot = None;
                }
                return;
            };

            let Some(position) = list.iter().position(|&i| i == vehicle_idx) else {
                if let Some(slot) = self.vehicle_lane_positions.get_mut(vehicle_idx) {
                    *slot = None;
                }
                return;
            };

            list.remove(position);
            (position, list.is_empty())
        };

        if remove_lane {
            self.vehicles_by_lane.remove(&lane);
        }

        if let Some(slot) = self.vehicle_lane_positions.get_mut(vehicle_idx) {
            *slot = None;
        }

        if let Some(list) = self.vehicles_by_lane.get(&lane) {
            for (slot, &other_idx) in list.iter().enumerate().skip(removed_slot) {
                if let Some(entry) = self.vehicle_lane_positions.get_mut(other_idx) {
                    *entry = Some(slot);
                }
            }
        }
    }

    fn start_vehicle_departure(&mut self, vidx: usize) -> bool {
        if self.vehicles[vidx].state != VehicleState::WaitingToDepart || self.vehicles[vidx].path.len() < 2 {
            return false;
        }

        let first_edge = match self.config.map.graph.find_edge(self.vehicles[vidx].path[0], self.vehicles[vidx].path[1]) {
            Some(edge) => edge,
            None => return false,
        };
        let lane_id = LaneId::Normal(first_edge, 0);

        let space_ok = self
            .vehicles_by_lane
            .get(&lane_id)
            .and_then(|lst| lst.first().copied())
            .is_none_or(|rear_idx| {
                self.vehicles[rear_idx].position_on_lane - self.vehicles[rear_idx].spec.length
                    >= self.config.minimum_gap
            });

        if !space_ok {
            return false;
        }

        self.vehicles[vidx].position_on_lane = 0.0;
        self.vehicles[vidx].state = VehicleState::OnRoad;
        self.vehicles[vidx].current_lane = Some(lane_id);

        self.insert_vehicle_into_lane(lane_id, vidx);

        if let Some(plan_id) = self.vehicles[vidx].commute_plan_id {
            if let Some(plan) = self.commute_plans.get_mut(&plan_id) {
                if plan.outbound_vehicle_id == self.vehicles[vidx].id {
                    plan.state = CommutePlanState::OutboundRunning;
                } else if plan.return_vehicle_id == self.vehicles[vidx].id {
                    plan.state = CommutePlanState::ReturnRunning;
                }
            }
        }
        true
    }
}

// Departures
impl SimulationEngine {
    fn handle_departures(&mut self) {
        let mut ready = Vec::new();

        while let Some(Reverse((departure_time, vidx))) = self.departure_queue.peek().copied() {
            if departure_time > OrderedFloat(self.current_time) {
                break;
            }

            self.departure_queue.pop();
            ready.push((departure_time, vidx));
        }

        let mut blocked = Vec::new();

        for (departure_time, vidx) in ready {
            if self.vehicles[vidx].state != VehicleState::WaitingToDepart {
                continue;
            }

            if OrderedFloat(self.vehicles[vidx].trip.departure_time) != departure_time {
                continue;
            }

            if !self.start_vehicle_departure(vidx) {
                blocked.push(vidx);
            }
        }

        for vidx in blocked {
            self.enqueue_departure(vidx);
        }
    }
}

// Plan movements

impl SimulationEngine {
    fn plan_movements(&mut self) {
        let lane_keys: Vec<LaneId> = self.vehicles_by_lane.keys().copied().collect();
        for lane_id in lane_keys {
            let mut cursor = 0usize;
            loop {
                let Some(vidx) = self
                    .vehicles_by_lane
                    .get(&lane_id)
                    .and_then(|indices| indices.get(cursor))
                    .copied()
                else {
                    break;
                };
                cursor += 1;

                if self.vehicles[vidx].state != VehicleState::OnRoad {
                    continue;
                }
                if matches!(self.vehicles[vidx].current_lane, Some(LaneId::Internal(_, _))) {
                    continue; // committed to crossing
                }
                
                self.rebuild_drive_plan(vidx);
            }
        }
    }

    fn rebuild_drive_plan(&mut self, vidx: usize) {
        let v = &self.vehicles[vidx];
        let a_max = v.spec.max_acceleration;
        let d_max = v.spec.comfortable_deceleration;
        let veh_len = v.spec.length;

        let braking_horizon = v.velocity * v.velocity / (2.0 * d_max) + v.velocity * 3.0 + 50.0;
        let path_index = v.path_index;
        let v0 = v.velocity;
        let t0 = self.current_time;

        let first_in_edge = match v.get_current_road(&self.config.map) {
            Some(e) => e,
            None => {
                self.vehicles[vidx].drive_plan.clear();
                return;
            }
        };
        let remaining = self.config.map.graph[first_in_edge].length - v.position_on_lane;

        let mut plan: Vec<DrivePlanEntry> = Vec::new();
        let mut dist_to_junction = remaining;
        let mut t_cursor = t0;
        let mut v_cursor = v0;
        let mut total_from_vehicle = 0.0f32;

        for i in path_index..v.path.len().saturating_sub(2) {
            total_from_vehicle += dist_to_junction;
            if total_from_vehicle > braking_horizon {
                break;
            }

            let from_node: NodeIndex = v.path[i];
            let junction_node: NodeIndex = v.path[i + 1];
            let to_node: NodeIndex = v.path[i + 2];

            let in_edge = match self.config.map.graph.find_edge(from_node, junction_node) {
                Some(e) => e,
                None => {continue;}
            };
            let out_edge = match self.config.map.graph.find_edge(junction_node, to_node) {
                Some(e) => e,
                None => {continue;}
            };

            let in_road_speed = self.config.map.graph[in_edge].speed_limit;
            let out_road_id = self.config.map.graph[out_edge].id;
            let out_road_len = self.config.map.graph[out_edge].length;
            let junction_id = self.config.map.graph[junction_node].id;

            let mut lane_idx = match self.vehicles[vidx].current_lane {
                Some(LaneId::Normal(e, lid)) if e == in_edge => lid as usize,
                _ => 0,
            };

            let mut link_opt = self.config.map.graph[in_edge]
                .lanes
                .get(lane_idx)
                .and_then(|lane| lane.links.iter().find(|l| l.destination_road_id == out_road_id))
                .map(|l| (l.id, l.via_internal_lane_id, l.link_type.clone()));

            // If not found in the presumed lane, scan other lanes on the same edge.
            if link_opt.is_none() {
                for (li, lane) in self.config.map.graph[in_edge].lanes.iter().enumerate() {
                    if let Some(l) = lane.links.iter().find(|l| l.destination_road_id == out_road_id) {
                        lane_idx = li;
                        link_opt = Some((l.id, l.via_internal_lane_id, l.link_type.clone()));
                        break;
                    }
                }
            }

            let (link_id, via_internal_lane_id, link_type) = match link_opt {
                Some(data) => data,
                None => continue,
            };

            let v1 = kinematics::approach_speed(&link_type, in_road_speed);
            let t_arrive =
                t_cursor + kinematics::arrival_time(dist_to_junction, v_cursor, v1, a_max, d_max);
            let v_leave = self.config.map.graph[out_edge].speed_limit;

            let (t_leave, il_len) = {
                match self.internal_lane_directory.get(&(junction_id, via_internal_lane_id)) {
                    Some(il) => (
                        kinematics::leave_time(t_arrive, il.length, veh_len, v1, v_leave),
                        il.length,
                    ),
                    None => (t_arrive + 1.0, 0.0),
                }
            };

            plan.push(DrivePlanEntry {
                link_id,
                lane_id: LaneId::Normal(in_edge, lane_idx as u32),
                via_internal_lane_id,
                junction_id,
                v_pass: v1.max(MIN_CREEP_SPEED),
                v_wait: kinematics::v_stop_at(total_from_vehicle, d_max),
                arrival_time: t_arrive,
                arrival_speed: v1,
                leave_time: t_leave,
                leave_speed: v_leave,
                distance: total_from_vehicle,
                set_request: true,
            });

            t_cursor = t_leave;
            v_cursor = v_leave;
            dist_to_junction = il_len + out_road_len;
        }

        self.vehicles[vidx].drive_plan = plan;
    }
}

// LC2013 lane changes

impl SimulationEngine {
    fn attempt_lane_changes(&mut self) {
        let mut pending: Vec<(usize, LaneId, LaneId)> = Vec::new();

        for vidx in 0..self.vehicles.len() {
            let (edge, lane_idx) = match self.vehicles[vidx].current_lane {
                Some(LaneId::Normal(e, li)) => (e, li as usize),
                _ => continue,
            };
            if self.vehicles[vidx].state != VehicleState::OnRoad {
                continue;
            }
            if self.vehicles[vidx].lane_change_cooldown > 0.0 {
                continue;
            }

            let lane_count = self.config.map.graph[edge].lanes.len();
            if lane_count < 2 {
                continue;
            }

            let next_road_id: Option<u32> = self.vehicles[vidx]
                .drive_plan
                .first()
                .and_then(|e| self.link_directory.get(&e.link_id))
                .map(|l| l.destination_road_id);

            let current_lane_id = LaneId::Normal(edge, lane_idx as u32);
            let (current_front_gap, current_front_vel) =
                self.vehicle_ahead_info(vidx, current_lane_id);

            let v_vel = self.vehicles[vidx].velocity;
            let v_len = self.vehicles[vidx].spec.length;
            let pos = self.vehicles[vidx].position_on_lane;

            let mut overtake_queued = false;

            // Try overtaking: move to lane_idx + 1
            if lane_idx + 1 < lane_count {
                let target_idx = lane_idx + 1;
                let target_lane_id = LaneId::Normal(edge, target_idx as u32);

                let strategically_ok = match next_road_id {
                    None => true,
                    Some(nrid) => self.config.map.graph[edge].lanes[target_idx]
                        .links
                        .iter()
                        .any(|l| l.destination_road_id == nrid),
                };

                if strategically_ok {
                    let (front_gap, front_vel) = self.vehicle_ahead_in_lane(target_lane_id, pos);
                    let (rear_gap, rear_vel) = self.vehicle_behind_in_lane(target_lane_id, pos, v_len);

                    let gap_safe = front_gap >= LC2013_SAFE_GAP_FRONT
                        && rear_gap >= LC2013_SAFE_GAP_REAR;
                    let follower_safe = rear_vel < 0.01
                        || (rear_vel * rear_vel) / (2.0 * rear_gap.max(0.01))
                            <= LC2013_MAX_DECEL_FOLLOWER;
                    // blocked: close to a slow leader whose speed is below desired
                    let v_desired = self.vehicles[vidx].spec.max_speed
                        .min(self.config.map.graph[edge].speed_limit);
                    let blocked = current_front_gap < 2.0 * LC2013_SAFE_GAP_FRONT
                        && v_desired > current_front_vel + LC2013_SPEED_GAIN_THRESHOLD;
                    // target advantageous: clear lane OR leader in target is faster than current
                    let target_advantageous = front_gap > 2.0 * LC2013_SAFE_GAP_FRONT
                        || front_vel > current_front_vel + LC2013_SPEED_GAIN_THRESHOLD;

                    if gap_safe && follower_safe && blocked && target_advantageous {
                        pending.push((vidx, current_lane_id, target_lane_id));
                        overtake_queued = true;
                    }
                }
            }

            // Keep-right: move to lane_idx - 1 if not overtaking
            if !overtake_queued && lane_idx > 0 {
                let target_idx = lane_idx - 1;
                let target_lane_id = LaneId::Normal(edge, target_idx as u32);

                let strategically_ok = match next_road_id {
                    None => true,
                    Some(nrid) => self.config.map.graph[edge].lanes[target_idx]
                        .links
                        .iter()
                        .any(|l| l.destination_road_id == nrid),
                };

                if strategically_ok {
                    let (front_gap, front_vel) = self.vehicle_ahead_in_lane(target_lane_id, pos);
                    let (rear_gap, rear_vel) = self.vehicle_behind_in_lane(target_lane_id, pos, v_len);

                    let gap_safe = front_gap >= LC2013_SAFE_GAP_FRONT
                        && rear_gap >= LC2013_SAFE_GAP_REAR;
                    let follower_safe = rear_vel < 0.01
                        || (rear_vel * rear_vel) / (2.0 * rear_gap.max(0.01))
                            <= LC2013_MAX_DECEL_FOLLOWER;
                    let right_ok = front_vel >= v_vel - LC2013_SPEED_GAIN_THRESHOLD
                        || front_gap > 2.0 * LC2013_SAFE_GAP_FRONT;

                    if gap_safe && follower_safe && right_ok {
                        pending.push((vidx, current_lane_id, target_lane_id));
                    }
                }
            }
        }

        // Cancel mutual swaps: two vehicles at similar positions crossing lanes would
        // pass through each other since partition_point uses strict < and misses same-pos pairs.
        let swap_threshold = LC2013_SAFE_GAP_FRONT;
        let mut cancelled: HashSet<usize> = HashSet::new();

        // Index pending by (from, to) for O(n) lookup instead of O(n²).
        let mut by_pair: HashMap<(LaneId, LaneId), Vec<(usize, f32)>> = HashMap::new();
        for (i, &(vidx, from, to)) in pending.iter().enumerate() {
            by_pair.entry((from, to)).or_default().push((i, self.vehicles[vidx].position_on_lane));
        }
        for (i, &(vidx, af, at)) in pending.iter().enumerate() {
            if let Some(opposites) = by_pair.get(&(at, af)) {
                let pa = self.vehicles[vidx].position_on_lane;
                for &(j, pb) in opposites {
                    if (pa - pb).abs() < swap_threshold {
                        cancelled.insert(i);
                        cancelled.insert(j);
                    }
                }
            }
        }

        for (idx, (vidx, from_lane, to_lane)) in pending.into_iter().enumerate() {
            if cancelled.contains(&idx) {
                continue;
            }
            if self.vehicles[vidx].current_lane != Some(from_lane) {
                continue;
            }
            // Re-check gaps against the live lane state (reflects moves already applied this step).
            let pos = self.vehicles[vidx].position_on_lane;
            let v_len = self.vehicles[vidx].spec.length;
            let (front_gap, _) = self.vehicle_ahead_in_lane(to_lane, pos);
            let (rear_gap, _) = self.vehicle_behind_in_lane(to_lane, pos, v_len);
            if front_gap < LC2013_SAFE_GAP_FRONT || rear_gap < LC2013_SAFE_GAP_REAR {
                continue;
            }
            self.remove_vehicle_from_lane(from_lane, vidx);
            self.insert_vehicle_into_lane(to_lane, vidx);
            self.vehicles[vidx].current_lane = Some(to_lane);
            self.vehicles[vidx].lane_change_cooldown = LC2013_COOLDOWN;
            self.rebuild_drive_plan(vidx);
        }
    }
}

// Register approaches

impl SimulationEngine {
    fn register_approaches(&mut self) {
        // let dt = self.config.time_step;

        for vidx in 0..self.vehicles.len() {
            if self.vehicles[vidx].state != VehicleState::OnRoad {
                continue;
            }

            let veh_id = self.vehicles[vidx].id;

            let old_ids = std::mem::take(&mut self.vehicles[vidx].registered_link_ids);
            for lid in old_ids {
                if let Some(s) = self.link_states.get_mut(&lid) {
                    s.approaching.remove(&veh_id);
                }
            }

            let plan_len = self.vehicles[vidx].drive_plan.len();
            for entry_idx in 0..plan_len {
                let (set_request, link_id, arrival_time, leave_time, arrival_speed, leave_speed) = {
                    let entry = &self.vehicles[vidx].drive_plan[entry_idx];
                    (
                        entry.set_request,
                        entry.link_id,
                        entry.arrival_time,
                        entry.leave_time,
                        entry.arrival_speed,
                        entry.leave_speed,
                    )
                };

                if !set_request {
                    continue;
                }
                // In case of deadlock, we can add a random jitter to the arrival and leave times.
                // let jitter = if rand::random::<bool>() { dt } else { 0.0 };
                let jitter = 0.0;
                let data = ApproachData {
                    arrival_time: arrival_time + jitter,
                    leave_time: leave_time + jitter,
                    arrival_speed: arrival_speed,
                    leave_speed: leave_speed,
                    will_pass: true,
                };
                self.link_states
                    .entry(link_id)
                    .or_default()
                    .approaching
                    .insert(veh_id, data);
                self.vehicles[vidx].registered_link_ids.push(link_id);
            }
        }
    }
}

// Traffic lights

impl SimulationEngine {
    fn advance_traffic_lights(&mut self) {
        let dt = self.config.time_step;
        let mut transitioned_controllers = Vec::new();

        for (&ctrl_id, state) in &mut self.traffic_light_states {
            let controller = match self.config.map.traffic_lights.get(&ctrl_id) {
                Some(c) => c,
                None => continue,
            };
            if controller.phases.is_empty() {
                continue;
            }

            let phase = &controller.phases[state.phase_index];
            let total_duration = phase.green_duration + phase.yellow_duration;

            state.time_in_phase += dt;
            if state.time_in_phase >= total_duration {
                state.time_in_phase -= total_duration;
                state.phase_index = (state.phase_index + 1) % controller.phases.len();
                transitioned_controllers.push(ctrl_id);
            }
        }

        for ctrl_id in transitioned_controllers {
            if let Some(previous_links) = self.controller_green_link_ids.remove(&ctrl_id) {
                for link_id in previous_links {
                    self.green_links.remove(&link_id);
                }
            }

            let state = match self.traffic_light_states.get(&ctrl_id) {
                Some(state) => state,
                None => continue,
            };

            let (active_links, active_roads) = Self::traffic_light_cache_for_controller(
                &self.config.map,
                &self.link_to_road_id,
                ctrl_id,
                state,
            );

            self.green_links.extend(active_links.iter().copied());
            self.controller_green_link_ids.insert(ctrl_id, active_links);
            self.controller_green_road_ids.insert(ctrl_id, active_roads);
        }
    }
}

// Execute movements

impl SimulationEngine {
    fn execute_movements(&mut self) {
        let lane_keys: Vec<LaneId> = self.vehicles_by_lane.keys().copied().collect();
        for lane_id in lane_keys {
            let mut cursor = 0usize;
            loop {
                let Some(vidx) = self
                    .vehicles_by_lane
                    .get(&lane_id)
                    .and_then(|indices| indices.get(cursor))
                    .copied()
                else {
                    break;
                };
                cursor += 1;

                if self.vehicles[vidx].state != VehicleState::OnRoad {
                    continue;
                }
                self.execute_vehicle(vidx, lane_id);
            }
        }
    }

    fn execute_vehicle(&mut self, vidx: usize, lane_id: LaneId) {
        let dt = self.config.time_step;

        let (safe_speed, stop_dist) = self.determine_safe_speed(vidx);

        let (mut ahead_dist, mut ahead_vel) = self.vehicle_ahead_info(vidx, lane_id);
        let speed_limit = self.lane_speed_limit(vidx);
        let desired;

        // Use IDM for stopping, setting the speed to 0 isn't working
        if let Some(dist) = stop_dist {
            desired = speed_limit;
            if dist < ahead_dist {
                ahead_dist = dist;
                ahead_vel = 0.0;
            }
        } else {
            desired = speed_limit.min(safe_speed);
        }

        let accel = self.vehicles[vidx].compute_acceleration(
            desired,
            self.config.minimum_gap,
            ahead_dist,
            ahead_vel,
        );

        {
            let v = &mut self.vehicles[vidx];
            v.velocity = (v.velocity + accel * dt).max(0.0);
            v.position_on_lane += v.velocity * dt;
            v.distance_traveled += v.velocity * dt;

            if v.velocity < 0.1 && !v.drive_plan.is_empty() {
                v.waiting_time += dt;
                v.impatience = (v.waiting_time * IMPATIENCE_RATE).min(1.0);
            } else if v.velocity > 0.5 {
                v.waiting_time = 0.0;
                v.impatience = 0.0;
            }
        }

        self.process_lane_advances(vidx);
    }

    fn determine_safe_speed(&self, vidx: usize) -> (f32, Option<f32>) {
        let v = &self.vehicles[vidx];

        if matches!(v.current_lane, Some(LaneId::Internal(_, _))) {
            return (v.spec.max_speed, None);
        }

        let entry = match v.drive_plan.first() {
            Some(e) => e,
            None => return (v.spec.max_speed, None),
        };

        let link = match self.find_link(entry.link_id) {
            Some(l) => l,
            None => return (v.spec.max_speed, None),
        };

        let ego = ApproachData {
            arrival_time: entry.arrival_time,
            leave_time: entry.leave_time,
            arrival_speed: entry.arrival_speed,
            leave_speed: entry.leave_speed,
            will_pass: true,
        };

        let link_open = is_link_open(
            &link,
            v,
            &ego,
            &self.link_states,
            &self.vehicles_by_lane,
            &self.vehicles,
            entry.junction_id,
            LOOK_AHEAD,
            STOP_DWELL_TIME,
            &self.green_links,
        );

        let mut space_ok = true;
        if link_open {
            if v.path_index + 2 < v.path.len() {
                if let Some(dest_edge) = self.config.map.graph.find_edge(v.path[v.path_index + 1], v.path[v.path_index + 2]) {
                    let to_lane = LaneId::Normal(dest_edge, link.lane_destination_id);
                    if let Some(list) = self.vehicles_by_lane.get(&to_lane) {
                        if let Some(&rear_idx) = list.first() {
                            let lv = &self.vehicles[rear_idx];
                            if lv.position_on_lane - lv.spec.length < self.config.minimum_gap {
                                space_ok = false;
                            }
                        }
                    }
                }
            }
        }

        if link_open && space_ok {
            // Point of no return: vehicle cannot decelerate to v_pass before the junction.
            let d_stop = v.velocity * v.velocity / (2.0 * v.spec.comfortable_deceleration);
            if entry.distance > 0.0 && entry.distance <= d_stop {
                return (v.spec.max_speed, None);
            }
            (entry.v_pass, None)
        } else {
            (entry.v_wait.max(0.0), Some(entry.distance))
        }

    }

    fn find_link(&self, link_id: u32) -> Option<&crate::map::road::Link> {
        self.link_directory.get(&link_id)
    }

    fn vehicle_ahead_info(&self, vidx: usize, lane_id: LaneId) -> (f32, f32) {
        let v = &self.vehicles[vidx];
        let lst = match self.vehicles_by_lane.get(&lane_id) {
            Some(l) => l,
            None => return (f32::INFINITY, v.spec.max_speed),
        };

        let my_slot = self
            .vehicle_lane_positions
            .get(vidx)
            .copied()
            .flatten()
            .or_else(|| lst.iter().position(|&i| i == vidx));
        let leader = my_slot.and_then(|p| lst.get(p + 1)).copied();

        match leader {
            Some(lidx) => {
                let lv = &self.vehicles[lidx];
                let gap = (lv.position_on_lane - lv.spec.length - v.position_on_lane).max(0.01);
                (gap, lv.previous_velocity)
            }
            None => (f32::INFINITY, 0.0)
        }
    }

    fn vehicle_ahead_in_lane(&self, lane_id: LaneId, pos: f32) -> (f32, f32) {
        let lst = match self.vehicles_by_lane.get(&lane_id) {
            Some(l) => l,
            None => return (f32::INFINITY, 0.0),
        };
        let split = lst.partition_point(|&i| self.vehicles[i].position_on_lane <= pos);
        match lst.get(split).copied() {
            Some(lidx) => {
                let lv = &self.vehicles[lidx];
                let gap = (lv.position_on_lane - lv.spec.length - pos).max(0.01);
                (gap, lv.previous_velocity)
            }
            None => (f32::INFINITY, 0.0),
        }
    }

    fn vehicle_behind_in_lane(&self, lane_id: LaneId, pos: f32, veh_len: f32) -> (f32, f32) {
        let lst = match self.vehicles_by_lane.get(&lane_id) {
            Some(l) => l,
            None => return (f32::INFINITY, 0.0),
        };
        let split = lst.partition_point(|&i| self.vehicles[i].position_on_lane < pos);
        if split == 0 {
            return (f32::INFINITY, 0.0);
        }
        let fidx = lst[split - 1];
        let fv = &self.vehicles[fidx];
        let gap = (pos - veh_len - fv.position_on_lane).max(0.0);
        (gap, fv.previous_velocity)
    }

    fn lane_length(&self, vidx: usize) -> f32 {
        match self.vehicles[vidx].current_lane {
            Some(LaneId::Normal(edge, _)) => self.config.map.graph[edge].length,
            Some(LaneId::Internal(jid, ilid)) => self
                .internal_lane_directory
                .get(&(jid, ilid))
                .map(|lane| lane.length)
                .unwrap_or(1.0),
            None => f32::INFINITY,
        }
    }

    fn lane_speed_limit(&self, vidx: usize) -> f32 {
        match self.vehicles[vidx].current_lane {
            Some(LaneId::Normal(edge, _)) => self.config.map.graph[edge].speed_limit,
            Some(LaneId::Internal(jid, ilid)) => self
                .internal_lane_directory
                .get(&(jid, ilid))
                .map(|lane| lane.speed_limit)
                .unwrap_or(crate::simulation::config::MAX_SPEED),
            None => crate::simulation::config::MAX_SPEED,
        }
    }

    fn process_lane_advances(&mut self, vidx: usize) {
        for _ in 0..16 {
            if self.vehicles[vidx].state == VehicleState::Arrived {
                break;
            }
            let lane_len = self.lane_length(vidx);
            if self.vehicles[vidx].position_on_lane < lane_len {
                break;
            }
            let current = match self.vehicles[vidx].current_lane {
                Some(l) => l,
                None => break,
            };
            match current {
                LaneId::Internal(_, _) => self.exit_internal_lane(vidx, current),
                LaneId::Normal(edge, _) => self.enter_junction_or_arrive(vidx, current, edge),
            }

        }
    }

    fn exit_internal_lane(&mut self, vidx: usize, from_lane: LaneId) {
        let il_len = self.lane_length(vidx);
        self.vehicles[vidx].position_on_lane -= il_len;
        self.vehicles[vidx].path_index += 1;

        let pi = self.vehicles[vidx].path_index;
        let current_node_idx = self.vehicles[vidx].path.get(pi).copied();

        // Check if current node is a waypoint and consume it
        if let Some(current_idx) = current_node_idx {
            if let Some(pos) = self.vehicles[vidx].waypoints.iter().position(|&w| w == current_idx) {
                
                self.vehicles[vidx].waypoints.remove(pos);
            }
        }
        
        if pi + 1 >= self.vehicles[vidx].path.len() {
            self.vehicles[vidx].state = VehicleState::Arrived;
            self.vehicles[vidx].arrived_at = Some(self.current_time);
            self.pending_transfers.push(PendingTransfer { vehicle_idx: vidx, from_lane, to_lane: None });
            self.handle_vehicle_arrival(vidx);
            return;
        }

        let a = self.vehicles[vidx].path[pi];
        let b = self.vehicles[vidx].path[pi + 1];
        let out_edge = match self.config.map.graph.find_edge(a, b) {
            Some(e) => e,
            None => return,
        };
        let dest_lane_id = match self.vehicles[vidx].current_lane {
            Some(LaneId::Internal(jid, ilid)) => {
                self.internal_lane_directory
                    .get(&(jid, ilid))
                    .map(|lane| lane.to_lane_id)
                    .unwrap_or(0)
            }
            _ => 0,
        };
        let to_lane = LaneId::Normal(out_edge, dest_lane_id);
        self.vehicles[vidx].current_lane = Some(to_lane);
        self.pending_transfers.push(PendingTransfer { vehicle_idx: vidx, from_lane, to_lane: Some(to_lane) });
    }

    fn enter_junction_or_arrive(&mut self, vidx: usize, from_lane: LaneId, in_edge: EdgeIndex) {
        let road_len = self.config.map.graph[in_edge].length;
        let pi = self.vehicles[vidx].path_index;
        let path_len = self.vehicles[vidx].path.len();

        if pi + 1 >= path_len - 1 {
            if matches!(self.vehicles[vidx].spec.kind, VehicleKind::Bus) {
                self.vehicles[vidx].state = VehicleState::WaitingToDepart;
                self.vehicles[vidx].path_index = 0;
                self.vehicles[vidx].position_on_lane = 0.0;
                self.vehicles[vidx].velocity = 0.0;
                self.vehicles[vidx].current_lane = None;
                self.vehicles[vidx].drive_plan.clear();
                self.vehicles[vidx].registered_link_ids.clear();
                self.vehicles[vidx].trip.departure_time = self.current_time;
                self.pending_transfers.push(PendingTransfer { vehicle_idx: vidx, from_lane, to_lane: None });
                self.enqueue_departure(vidx);
                return;
            }
            self.vehicles[vidx].position_on_lane = road_len;
            self.vehicles[vidx].state = VehicleState::Arrived;
            self.vehicles[vidx].arrived_at = Some(self.current_time);
            self.pending_transfers.push(PendingTransfer { vehicle_idx: vidx, from_lane, to_lane: None });
            self.handle_vehicle_arrival(vidx);
            return;
        }

        self.vehicles[vidx].position_on_lane -= road_len;

        let via_internal_lane_id = match self.vehicles[vidx].drive_plan.first() {
            Some(entry) => entry.via_internal_lane_id,
            None => {
                // No drive plan -> stay stopped until next rebuild
                self.vehicles[vidx].position_on_lane = 0.0;
                self.vehicles[vidx].velocity = 0.0;
                return;
            }
        };

        let junction_node: NodeIndex = self.vehicles[vidx].path[pi + 1];
        let junction_id = self.config.map.graph[junction_node].id;
        // If the junction node is a waypoint, consume it now.
        if let Some(pos) = self.vehicles[vidx].waypoints.iter().position(|&w| w == junction_node) {
            self.vehicles[vidx].waypoints.remove(pos);
        }
        let to_lane = LaneId::Internal(junction_id, via_internal_lane_id);

        self.vehicles[vidx].current_lane = Some(to_lane);
        self.vehicles[vidx].drive_plan.remove(0);
        self.pending_transfers.push(PendingTransfer { vehicle_idx: vidx, from_lane, to_lane: Some(to_lane) });
    }

    fn check_if_all_vehicles_arrived(&mut self) {
        let non_bus: Vec<_> = self.vehicles.iter().filter(|v| !matches!(v.spec.kind, VehicleKind::Bus)).collect();
        let nb_arrived = non_bus.iter().filter(|v| matches!(v.state, VehicleState::Arrived)).count();
        self.all_vehicles_arrived = !non_bus.is_empty() && nb_arrived == non_bus.len();
    }

    fn handle_vehicle_arrival(&mut self, vidx: usize) {
        let Some(plan_id) = self.vehicles[vidx].commute_plan_id else {
            self.check_if_all_vehicles_arrived();
            return;
        };

        let snapshot = match self.commute_plans.get(&plan_id) {
            Some(plan) => (
                plan.state,
                plan.outbound_vehicle_id,
                plan.return_vehicle_id,
                plan.return_waiting_time_s,
            ),
            None => {
                self.check_if_all_vehicles_arrived();
                return;
            }
        };

        let vehicle_id = self.vehicles[vidx].id;

        match snapshot.0 {
            CommutePlanState::OutboundPending | CommutePlanState::OutboundRunning
                if vehicle_id == snapshot.1 => {
                let ready_at = self.vehicles[vidx].arrived_at.unwrap_or(self.current_time) + snapshot.3;

                let return_idx = match self.vehicle_indices_by_id.get(&snapshot.2).copied() {
                    Some(index) => index,
                    None => {
                        if let Some(plan) = self.commute_plans.get_mut(&plan_id) {
                            plan.state = CommutePlanState::Failed;
                        }
                        self.check_if_all_vehicles_arrived();
                        return;
                    }
                };

                let path_ok = {
                    let return_vehicle = &mut self.vehicles[return_idx];
                    return_vehicle.trip.departure_time = ready_at;
                    return_vehicle.state = VehicleState::WaitingToDepart;
                    return_vehicle.position_on_lane = 0.0;
                    return_vehicle.velocity = 0.0;
                    return_vehicle.previous_velocity = 0.0;
                    return_vehicle.current_lane = None;
                    return_vehicle.drive_plan.clear();
                    return_vehicle.registered_link_ids.clear();
                    return_vehicle.waiting_time = 0.0;
                    return_vehicle.impatience = 0.0;
                    return_vehicle.update_path(&self.config.map)
                };

                if let Some(plan) = self.commute_plans.get_mut(&plan_id) {
                    plan.state = if path_ok {
                        CommutePlanState::WaitingForReturnDeparture
                    } else {
                        CommutePlanState::Failed
                    };
                }

                if !path_ok {
                    self.vehicles[return_idx].trip.departure_time = f32::MAX;
                } else {
                    self.enqueue_departure(return_idx);
                }
            }
            CommutePlanState::ReturnRunning if vehicle_id == snapshot.2 => {
                if let Some(plan) = self.commute_plans.get_mut(&plan_id) {
                    plan.state = CommutePlanState::Completed;
                }
            }
            _ => {}
        }

        self.check_if_all_vehicles_arrived();
    }

}

// Buffer flush

impl SimulationEngine {
    pub fn add_bus_from_saved(&mut self, bl: &crate::map::model::SavedBusLine) -> bool {
        let map = self.config.map.clone();
        let stops: Vec<_> = bl.stop_node_ids.iter()
            .filter_map(|nid| map.node_index_map.get(nid).copied())
            .collect();
        if stops.len() < 2 {
            return false;
        }
        let spec = VehicleSpec::new(VehicleKind::Bus, 8.0, 2.0, 3.0, 1.0, 12.0);
        let trip = TripRequest { origin: stops[0], destination: stops[0], departure_time: 0.0 };
        let vehicle_id = self.next_vehicle_id;
        self.next_vehicle_id += 1;
        let mut bus = Vehicle::new(vehicle_id, spec, trip, VehicleType::Electrique);
        bus.waypoints = stops[1..].to_vec();
        bus.update_path(&map);
        if bus.path.len() < 2 {
            return false;
        }
        let vidx = self.vehicles.len();
        self.vehicles.push(bus);
        self.vehicle_lane_positions.push(None);
        self.vehicle_indices_by_id.insert(vehicle_id, vidx);
        self.bus_lines.push(BusLine { id: bl.id, name: bl.name.clone(), stop_node_ids: bl.stop_node_ids.clone(), vehicle_id });
        self.enqueue_departure(vidx);
        true
    }
}

impl SimulationEngine {
    fn flush_transfers(&mut self) {
        let transfers: Vec<PendingTransfer> = self.pending_transfers.drain(..).collect();
        for t in transfers {
            self.remove_vehicle_from_lane(t.from_lane, t.vehicle_idx);
            if let Some(to_lane) = t.to_lane {
                if self.vehicles[t.vehicle_idx].state != VehicleState::Arrived {
                    self.insert_vehicle_into_lane(to_lane, t.vehicle_idx);
                }
            }
        }
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
