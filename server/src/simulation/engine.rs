use std::collections::{HashMap, HashSet};

use crate::scoring;
use crate::simulation::config::{
    SimulationConfig, IMPATIENCE_RATE, LOOK_AHEAD, MIN_CREEP_SPEED, STOP_DWELL_TIME,
};
use crate::map::intersection::{ApproachData, LinkState, is_link_open};
use crate::map::road::LinkType;
use crate::simulation::kinematics;
use crate::simulation::vehicle::{DrivePlanEntry, LaneId, Vehicle, VehicleState};
use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;

pub trait Simulation {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self;
    fn run(&mut self);
    fn step(&mut self);
    fn get_score(&self) -> f32;
}

struct PendingTransfer {
    vehicle_idx: usize,
    from_lane: LaneId,
    to_lane: Option<LaneId>,
}

struct TrafficLightRuntimeState {
    phase_index: usize,
    time_in_phase: f32,
}

pub struct SimulationEngine {
    pub config: SimulationConfig,
    pub vehicles: Vec<Vehicle>,
    pub current_time: f32,
    pub vehicles_by_lane: HashMap<LaneId, Vec<usize>>, // Sorted by position_on_lane (back → front = index 0 first).
    pub link_states: HashMap<u32, LinkState>,
    pub green_links: HashSet<u32>,
    pending_transfers: Vec<PendingTransfer>,
    traffic_light_states: HashMap<u32, TrafficLightRuntimeState>,
}

impl Simulation for SimulationEngine {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self {
        let current_time = config.start_time;
        let traffic_light_states = config
            .map
            .traffic_lights
            .keys()
            .map(|&id| (id, TrafficLightRuntimeState { phase_index: 0, time_in_phase: 0.0 }))
            .collect();
        Self {
            config,
            vehicles,
            current_time,
            vehicles_by_lane: HashMap::new(),
            link_states: HashMap::new(),
            green_links: HashSet::new(),
            pending_transfers: Vec::new(),
            traffic_light_states,
        }
    }

    fn run(&mut self) {
        for v in &mut self.vehicles {
            v.update_path(&self.config.map);
        }
        while self.current_time < self.config.end_time {
            self.step();
            self.current_time += self.config.time_step;
        }
    }
  
    fn get_score(&self) -> f32 {
        scoring::compute_score(&self.vehicles, &self.config)
    }

    fn step(&mut self) {
        for v in &mut self.vehicles {
            v.previous_velocity = v.velocity;
        }
        self.handle_departures();
        self.plan_movements();
        self.register_approaches();
        self.advance_traffic_lights();
        self.execute_movements();
        self.flush_transfers();
        let dt = self.config.time_step;
        let t = self.current_time;
        for v in &mut self.vehicles {
            if v.state == VehicleState::OnRoad {
                scoring::update_co2_emissions(v, dt);
            }
            if v.state == VehicleState::Arrived && v.arrived_at.is_none() {
                v.arrived_at = Some(t);
            }
        }

        let _ = self.solve_interblocking();

        // Debug: log overlapping vehicles per lane at end of each step
        self.log_overlaps();
    }

}

// Departures
impl SimulationEngine {
    fn log_overlaps(&self) {
        for (lane, lst) in &self.vehicles_by_lane {
            if lst.len() < 2 {
                // no possible overlap
                continue;
            }
            let mut overlapped: HashSet<usize> = HashSet::new();
            for pair in lst.windows(2) {
                let rear_idx = pair[0];
                let ahead_idx = pair[1];
                let rear = &self.vehicles[rear_idx];
                let ahead = &self.vehicles[ahead_idx];
                let rear_front = rear.position_on_lane;
                let ahead_back = ahead.position_on_lane - ahead.spec.length;
                if rear_front > ahead_back {
                    overlapped.insert(rear_idx);
                    overlapped.insert(ahead_idx);
                }
            }
            if !overlapped.is_empty() {
                match lane {
                    LaneId::Internal(_jid, _ilid) => {
                        // overlap logging suppressed
                    }
                    LaneId::Normal(_edge, _lid) => {
                        // overlap logging suppressed
                    }
                }
            }
        }
    }
    fn handle_departures(&mut self) {
        let waiting: Vec<usize> = self
            .vehicles
            .iter()
            .enumerate()
            .filter(|(_, v)| v.state == VehicleState::WaitingToDepart && v.path.len() >= 2)
            .map(|(i, _)| i)
            .collect();

        for vidx in waiting {
            let first_edge = {
                let v = &self.vehicles[vidx];
                match self.config.map.graph.find_edge(v.path[0], v.path[1]) {
                    Some(e) => e,
                    None => continue,
                }
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
                continue;
            }

            self.vehicles[vidx].position_on_lane = 0.0;
            self.vehicles[vidx].state = VehicleState::OnRoad;
            self.vehicles[vidx].current_lane = Some(lane_id);
            
            lane_insert_sorted(&mut self.vehicles_by_lane, &self.vehicles, lane_id, vidx);
        }
    }
}

// Plan movements

impl SimulationEngine {
    fn plan_movements(&mut self) {
        let lane_keys: Vec<LaneId> = self.vehicles_by_lane.keys().copied().collect();
        for lane_id in lane_keys {
            let indices: Vec<usize> = self
                .vehicles_by_lane
                .get(&lane_id)
                .cloned()
                .unwrap_or_default();
            for &vidx in &indices {
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
                None => break,
            };
            let out_edge = match self.config.map.graph.find_edge(junction_node, to_node) {
                Some(e) => e,
                None => break,
            };

            let in_road_speed = self.config.map.graph[in_edge].speed_limit;
            let out_road_id = self.config.map.graph[out_edge].id;
            let out_road_len = self.config.map.graph[out_edge].length;
            let junction_id = self.config.map.graph[junction_node].id;

            let lane_idx = match self.vehicles[vidx].current_lane {
                Some(LaneId::Normal(e, lid)) if e == in_edge => lid as usize,
                _ => 0,
            };
            let link = self.config.map.graph[in_edge]
                .lanes
                .get(lane_idx)
                .and_then(|lane| lane.links.iter().find(|l| l.destination_road_id == out_road_id))
                .cloned();

            let link = match link {
                Some(l) => l,
                None => break,
            };

            let v1 = kinematics::approach_speed(&link.link_type, in_road_speed);
            let t_arrive =
                t_cursor + kinematics::arrival_time(dist_to_junction, v_cursor, v1, a_max, d_max);
            let v_leave = self.config.map.graph[out_edge].speed_limit;

            let (t_leave, il_len) = {
                let jnode = &self.config.map.graph[junction_node];
                match jnode.internal_lanes.iter().find(|il| il.id == link.via_internal_lane_id) {
                    Some(il) => (
                        kinematics::leave_time(t_arrive, il.length, veh_len, v1, v_leave),
                        il.length,
                    ),
                    None => (t_arrive + 1.0, 0.0),
                }
            };

            plan.push(DrivePlanEntry {
                link_id: link.id,
                lane_id: LaneId::Normal(in_edge, lane_idx as u32),
                via_internal_lane_id: link.via_internal_lane_id,
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

// Register approaches

impl SimulationEngine {
    fn register_approaches(&mut self) {
        // let dt = self.config.time_step;

        for vidx in 0..self.vehicles.len() {
            if self.vehicles[vidx].state != VehicleState::OnRoad {
                continue;
            }

            let veh_id = self.vehicles[vidx].id;

            let old_ids: Vec<u32> = self.vehicles[vidx].registered_link_ids.clone();
            for lid in old_ids {
                if let Some(s) = self.link_states.get_mut(&lid) {
                    s.approaching.remove(&veh_id);
                }
            }
            self.vehicles[vidx].registered_link_ids.clear();

            let plan: Vec<DrivePlanEntry> = self.vehicles[vidx].drive_plan.clone();
            for entry in plan {
                if !entry.set_request {
                    continue;
                }
                // In case of deadlock, we can add a random jitter to the arrival and leave times.
                // let jitter = if rand::random::<bool>() { dt } else { 0.0 };
                let jitter = 0.0;
                let data = ApproachData {
                    arrival_time: entry.arrival_time + jitter,
                    leave_time: entry.leave_time + jitter,
                    arrival_speed: entry.arrival_speed,
                    leave_speed: entry.leave_speed,
                    will_pass: true,
                };
                self.link_states
                    .entry(entry.link_id)
                    .or_default()
                    .approaching
                    .insert(veh_id, data);
                self.vehicles[vidx].registered_link_ids.push(entry.link_id);
            }
        }
    }
}

// Traffic lights

impl SimulationEngine {
    fn advance_traffic_lights(&mut self) {
        let dt = self.config.time_step;
        self.green_links.clear();

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
            }

            let current_phase = &controller.phases[state.phase_index];
            if state.time_in_phase < current_phase.green_duration {
                let ids: Vec<u32> = current_phase.green_link_ids.iter().copied().collect();
                self.green_links.extend(ids);
            }
        }
    }
}

// Execute movements

impl SimulationEngine {
    fn execute_movements(&mut self) {
        let lane_keys: Vec<LaneId> = self.vehicles_by_lane.keys().copied().collect();
        for lane_id in lane_keys {
            let indices: Vec<usize> = self
                .vehicles_by_lane
                .get(&lane_id)
                .cloned()
                .unwrap_or_default();
            for &vidx in &indices {
                if self.vehicles[vidx].state != VehicleState::OnRoad {
                    continue;
                }
                self.execute_vehicle(vidx, lane_id);
            }
        }
    }

    fn execute_vehicle(&mut self, vidx: usize, lane_id: LaneId) {
        let dt = self.config.time_step;

        let (safe_speed, _stop_dist) = self.determine_safe_speed(vidx);

        let (ahead_dist, ahead_vel) = self.vehicle_ahead_info(vidx, lane_id);
        let speed_limit = self.lane_speed_limit(vidx);
        let desired;

        // Use IDM for stopping, but keep automatic braking disabled for now.
        // The original braking logic that considered `stop_dist` is commented
        // below for debugging and to avoid vehicles stopping slightly before
        // junctions.
        // if let Some(dist) = stop_dist {
        //     desired = speed_limit;
        //     if dist < ahead_dist {
        //         ahead_dist = dist;
        //         ahead_vel = 0.0;
        //     }
        // } else {
        //     desired = speed_limit.min(safe_speed);
        // }
        desired = speed_limit.min(safe_speed);

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

        if is_link_open(
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
        ) {
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

    fn find_link(&self, link_id: u32) -> Option<crate::map::road::Link> {
        for edge in self.config.map.graph.edge_indices() {
            for lane in &self.config.map.graph[edge].lanes {
                if let Some(lnk) = lane.links.iter().find(|l| l.id == link_id) {
                    return Some(lnk.clone());
                }
            }
        }
        None
    }

    fn vehicle_ahead_info(&self, vidx: usize, lane_id: LaneId) -> (f32, f32) {
        let v = &self.vehicles[vidx];
        let lst = match self.vehicles_by_lane.get(&lane_id) {
            Some(l) => l,
            None => return (f32::INFINITY, v.spec.max_speed),
        };

        let my_slot = lst.iter().position(|&i| i == vidx);
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

    fn lane_length(&self, vidx: usize) -> f32 {
        match self.vehicles[vidx].current_lane {
            Some(LaneId::Normal(edge, _)) => self.config.map.graph[edge].length,
            Some(LaneId::Internal(jid, ilid)) => self
                .config
                .map
                .node_index_map
                .get(&jid)
                .and_then(|&ni| {
                    self.config.map.graph[ni]
                        .internal_lanes
                        .iter()
                        .find(|il| il.id == ilid)
                        .map(|il| il.length)
                })
                .unwrap_or(1.0),
            None => f32::INFINITY,
        }
    }

    fn lane_speed_limit(&self, vidx: usize) -> f32 {
        match self.vehicles[vidx].current_lane {
            Some(LaneId::Normal(edge, _)) => self.config.map.graph[edge].speed_limit,
            Some(LaneId::Internal(jid, ilid)) => self
                .config
                .map
                .node_index_map
                .get(&jid)
                .and_then(|&ni| {
                    self.config.map.graph[ni]
                        .internal_lanes
                        .iter()
                        .find(|il| il.id == ilid)
                        .map(|il| il.speed_limit)
                })
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
        let original_pos = self.vehicles[vidx].position_on_lane;
        let entering_pos = original_pos - il_len;

        // Compute the indices we'd use if we were to advance the path index.
        let pi_next = self.vehicles[vidx].path_index + 1;
        if pi_next + 1 >= self.vehicles[vidx].path.len() {
            // No further edge after the next node -> arriving at destination when exiting.
            self.vehicles[vidx].state = VehicleState::Arrived;
            self.vehicles[vidx].current_lane = None;
            self.vehicles[vidx].velocity = 0.0;
            
            self.pending_transfers.push(PendingTransfer { vehicle_idx: vidx, from_lane, to_lane: None });
            return;
        }

        let a = self.vehicles[vidx].path[pi_next];
        let b = self.vehicles[vidx].path[pi_next + 1];
        let out_edge = match self.config.map.graph.find_edge(a, b) {
            Some(e) => e,
            None => return,
        };
        let dest_lane_id = match self.vehicles[vidx].current_lane {
            Some(LaneId::Internal(jid, ilid)) => {
                if let Some(&ni) = self.config.map.node_index_map.get(&jid) {
                    self.config.map.graph[ni]
                        .internal_lanes
                        .iter()
                        .find(|il| il.id == ilid)
                        .map(|il| il.to_lane_id)
                        .unwrap_or(0)
                } else {
                    0
                }
            }
            _ => 0,
        };
        let to_lane = LaneId::Normal(out_edge, dest_lane_id);

        // Check whether there is space in the normal lane at `entering_pos`.
        // If not enough space, keep the vehicle at the end of the internal lane
        // (do not transfer yet) so it doesn't create overlaps in the normal lane.
        if !self.can_enter_normal_lane(to_lane, entering_pos, vidx) {
            self.vehicles[vidx].position_on_lane = il_len; // stay at end of internal lane
            self.vehicles[vidx].velocity = 0.0;
            
            return;
        }

        // Commit transfer into the normal lane.
        self.vehicles[vidx].position_on_lane = entering_pos;
        self.vehicles[vidx].current_lane = Some(to_lane);
        // Advance the path index now that we've left the internal lane.
        self.vehicles[vidx].path_index = pi_next;
        // Immediately reserve/insert the vehicle into the destination normal lane so
        // subsequent transfers or entries see the reservation and avoid stacking.
        lane_insert_sorted(&mut self.vehicles_by_lane, &self.vehicles, to_lane, vidx);
        
        self.pending_transfers.push(PendingTransfer { vehicle_idx: vidx, from_lane, to_lane: Some(to_lane) });
    }

    fn enter_junction_or_arrive(&mut self, vidx: usize, from_lane: LaneId, in_edge: EdgeIndex) {
        let road_len = self.config.map.graph[in_edge].length;
        let pi = self.vehicles[vidx].path_index;
        let path_len = self.vehicles[vidx].path.len();

        if pi + 1 >= path_len - 1 {
            self.vehicles[vidx].position_on_lane = road_len;
            self.vehicles[vidx].state = VehicleState::Arrived;
            self.vehicles[vidx].current_lane = None;
            self.vehicles[vidx].velocity = 0.0;
            
            self.pending_transfers.push(PendingTransfer { vehicle_idx: vidx, from_lane, to_lane: None });
            return;
        }
        self.vehicles[vidx].position_on_lane -= road_len;

        let entry = match self.vehicles[vidx].drive_plan.first().cloned() {
            Some(e) => e,
            None => {
                self.vehicles[vidx].position_on_lane = 0.0;
                self.vehicles[vidx].velocity = 0.0;
                return;
            }
        };

        let junction_node: NodeIndex = self.vehicles[vidx].path[pi + 1];
        let junction_id = self.config.map.graph[junction_node].id;
        let to_lane = LaneId::Internal(junction_id, entry.via_internal_lane_id);

        // Decide whether we can enter the internal lane:
        // - if the internal lane is empty -> allow immediately
        // - otherwise check gaps to nearest leader and follower against `minimum_gap`
        let entering_pos = self.vehicles[vidx].position_on_lane;
        if !self.can_enter_internal_lane(to_lane, entering_pos, vidx) {
            // Not enough space to enter the internal lane: stay at end of current road.
            self.vehicles[vidx].position_on_lane = road_len;
            self.vehicles[vidx].velocity = 0.0;
            return;
        }

        self.vehicles[vidx].current_lane = Some(to_lane);
        self.vehicles[vidx].drive_plan.remove(0);
        // Immediately reserve/insert the vehicle into the target internal lane so
        // subsequent calls to `can_enter_internal_lane` see the reservation and
        // cannot allow other vehicles to enter (prevents stacking).
        lane_insert_sorted(&mut self.vehicles_by_lane, &self.vehicles, to_lane, vidx);
        
        self.pending_transfers.push(PendingTransfer { vehicle_idx: vidx, from_lane, to_lane: Some(to_lane) });
    }

    // Helper: returns true if the internal lane has no vehicles registered.
    fn internal_lane_is_empty(&self, lane: &LaneId) -> bool {
        match self.vehicles_by_lane.get(lane) {
            None => true,
            Some(v) => v.is_empty(),
        }
    }

    // Helper: find nearest leader (smallest position >= entering_pos) and nearest follower (largest position < entering_pos)
    // in the given lane. Returns (leader_idx_opt, follower_idx_opt).
    fn nearest_leader_and_follower(&self, lane: &LaneId, entering_pos: f32) -> (Option<usize>, Option<usize>) {
        let mut leader: Option<usize> = None;
        let mut follower: Option<usize> = None;
        if let Some(lst) = self.vehicles_by_lane.get(lane) {
            for &i in lst.iter() {
                let p = self.vehicles[i].position_on_lane;
                // Treat vehicles at exactly entering_pos as leaders to avoid
                // allowing an entering vehicle to occupy the exact same
                // slot as an existing one.
                if p >= entering_pos {
                    match leader {
                        None => leader = Some(i),
                        Some(prev) => {
                            if p < self.vehicles[prev].position_on_lane {
                                leader = Some(i);
                            }
                        }
                    }
                } else if p < entering_pos {
                    match follower {
                        None => follower = Some(i),
                        Some(prev) => {
                            if p > self.vehicles[prev].position_on_lane {
                                follower = Some(i);
                            }
                        }
                    }
                }
            }
        }
        (leader, follower)
    }

    // Check whether the vehicle `vidx` at `entering_pos` can enter `to_lane` respecting `minimum_gap`.
    fn can_enter_internal_lane(&self, to_lane: LaneId, entering_pos: f32, vidx: usize) -> bool {
        if self.internal_lane_is_empty(&to_lane) {
            return true;
        }

        let min_gap = self.config.minimum_gap;

        let (leader_opt, follower_opt) = self.nearest_leader_and_follower(&to_lane, entering_pos);

        // Check leader gap: leader.position_on_lane - leader.length - entering_pos >= min_gap
        if let Some(li) = leader_opt {
            let leader = &self.vehicles[li];
            let gap = leader.position_on_lane - leader.spec.length - entering_pos;
            if gap < min_gap {
                return false;
            }
        }

        // Check follower gap: entering_pos - entering_vehicle.length - follower.position_on_lane >= min_gap
        if let Some(fi) = follower_opt {
            let follower = &self.vehicles[fi];
            let gap = entering_pos - self.vehicles[vidx].spec.length - follower.position_on_lane;
            if gap < min_gap {
                return false;
            }
        }

        true
    }

    // Check whether the vehicle `vidx` at `entering_pos` can enter a normal lane
    // respecting the minimum gap and ensuring the vehicle rear is within the lane bounds.
    fn can_enter_normal_lane(&self, to_lane: LaneId, entering_pos: f32, vidx: usize) -> bool {
        let min_gap = self.config.minimum_gap;

        // Allow the entering vehicle's rear to be negative (it may still be
        // partially inside the junction). Decide using explicit interval
        // checks against leader and follower, with `minimum_gap` padding.
        let entering_front = entering_pos;
        let entering_rear = entering_pos - self.vehicles[vidx].spec.length;

        let (leader_opt, follower_opt) = self.nearest_leader_and_follower(&to_lane, entering_pos);

        if let Some(li) = leader_opt {
            let leader = &self.vehicles[li];
            let leader_back = leader.position_on_lane - leader.spec.length;
            // require leader_back - entering_front >= min_gap
            if leader_back - entering_front < min_gap {
                return false;
            }
        }

        if let Some(fi) = follower_opt {
            let follower = &self.vehicles[fi];
            let follower_front = follower.position_on_lane;
            // require entering_rear - follower_front >= min_gap
            if entering_rear - follower_front < min_gap {
                return false;
            }
        }

        true
    }
}

// Buffer flush

impl SimulationEngine {
    fn flush_transfers(&mut self) {
        let transfers: Vec<PendingTransfer> = self.pending_transfers.drain(..).collect();
        for t in transfers {
            if let Some(lst) = self.vehicles_by_lane.get_mut(&t.from_lane) {
                lst.retain(|&i| i != t.vehicle_idx);
                if lst.is_empty() {
                    self.vehicles_by_lane.remove(&t.from_lane);
                }
            }
            if let Some(to_lane) = t.to_lane {
                if self.vehicles[t.vehicle_idx].state != VehicleState::Arrived {
                    // If the vehicle was already inserted as a reservation when
                    // authorizing the transfer, avoid inserting it again.
                    let already_present = self
                        .vehicles_by_lane
                        .get(&to_lane)
                        .map_or(false, |lst| lst.contains(&t.vehicle_idx));
                    if !already_present {
                        lane_insert_sorted(&mut self.vehicles_by_lane, &self.vehicles, to_lane, t.vehicle_idx);
                    }
                }
            }
        }
        // Log overlaps again after transfers are flushed so we can see
        // any collisions introduced/avoided by transfer operations.
        self.log_overlaps();
    }
}

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

impl SimulationEngine {
    /// Detect whether there is a priority-cycle at the given junction.
    ///
    /// We consider all `Link`s that target `junction_id` and that have at least
    /// one approaching vehicle registered in `self.link_states`. For each such
    /// link `L` we add directed edges `L -> F` for every foe `F` of `L` that
    /// (a) has an approaching vehicle and (b) actually imposes a yield on `L`
    /// according to the same rules used by `is_link_open` (including ignoring
    /// foe traffic-lights that are not currently green). If the resulting
    /// directed graph contains a cycle then we report `true`.
    pub fn junction_has_priority_cycle(&self, junction_id: u32) -> bool {
        use petgraph::Direction;

        let junction_node = match self.config.map.node_index_map.get(&junction_id) {
            Some(&ni) => ni,
            None => return false,
        };

        // Collect all links incoming to this junction and mark which have approaching vehicles.
        let mut links_map: HashMap<u32, crate::map::road::Link> = HashMap::new();
        let mut active_links: HashSet<u32> = HashSet::new();

        for edge in self.config.map.graph.edges_directed(junction_node, Direction::Incoming) {
            for lane in &edge.weight().lanes {
                for l in &lane.links {
                    links_map.insert(l.id, l.clone());
                    if self.link_states.get(&l.id).map_or(false, |s| !s.approaching.is_empty()) {
                        active_links.insert(l.id);
                    }
                }
            }
        }

        if active_links.is_empty() {
            return false;
        }

        // Build adjacency list of "must yield to" relations among active links.
        let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();
        for &lid in &active_links {
            if let Some(ego) = links_map.get(&lid) {
                for foe in &ego.foe_links {
                    // If the foe is a traffic light and is not green, it does not block.
                    if foe.link_type == LinkType::TrafficLight && !self.green_links.contains(&foe.id) {
                        continue;
                    }

                    // Consider only foes that are active (have approaching vehicles).
                    if !active_links.contains(&foe.id) {
                        continue;
                    }

                    let must_yield = match (&ego.link_type, &foe.link_type) {
                        (LinkType::Priority, LinkType::Yield) | (LinkType::Priority, LinkType::Stop) => false,
                        (LinkType::Yield, LinkType::Priority) | (LinkType::Stop, LinkType::Priority) => true,
                        _ => crate::map::intersection::foe_is_to_the_right(ego, foe),
                    };

                    if must_yield {
                        adj.entry(lid).or_default().push(foe.id);
                    }
                }
            }
        }

        // Detect directed cycle in adj using DFS (white/gray/black).
        #[derive(Copy, Clone, PartialEq, Eq)]
        enum Color { White, Gray, Black }
        let mut color: HashMap<u32, Color> = HashMap::new();
        for &n in &active_links { color.insert(n, Color::White); }

        fn dfs(u: u32, adj: &HashMap<u32, Vec<u32>>, color: &mut HashMap<u32, Color>) -> bool {
            color.insert(u, Color::Gray);
            if let Some(neis) = adj.get(&u) {
                for &v in neis {
                    match color.get(&v).copied().unwrap_or(Color::White) {
                        Color::White => {
                            if dfs(v, adj, color) { return true; }
                        }
                        Color::Gray => return true, // found cycle
                        Color::Black => {}
                    }
                }
            }
            color.insert(u, Color::Black);
            false
        }

        for &n in &active_links {
            if color.get(&n) == Some(&Color::White) {
                if dfs(n, &adj, &mut color) { return true; }
            }
        }

        false
    }

    /// Scan all junctions and try to resolve priority-cycles by forcing a
    /// vehicle into an internal lane when possible. Returns `true` if at
    /// least one cycle was resolved.
    pub fn solve_interblocking(&mut self) -> bool {
        let mut resolved_any = false;
        for node in self.config.map.graph.node_indices() {
            let junction_id = self.config.map.graph[node].id;

            // If this junction currently has a priority-cycle, increment
            // its `interblocked_for` timer by the simulation timestep and
            // only attempt resolution after it exceeds 1.0s. Otherwise
            // reset the timer.
            if self.junction_has_priority_cycle(junction_id) {
                {
                    let node_mut = &mut self.config.map.graph[node];
                    node_mut.interblocked_for += self.config.time_step;
                    if node_mut.interblocked_for <= 2.0 {
                        continue;
                    }
                }
            } else {
                let node_mut = &mut self.config.map.graph[node];
                node_mut.interblocked_for = 0.0;
                continue;
            }

            // At this point the junction has been blocked for > 1s; proceed
            // to attempt resolution. Use an immutable reference to the map
            // for read-only access below.
            let map = &self.config.map;

            // logging suppressed: junction blocked info removed

            // We'll decide per-candidate vehicle whether to attempt forcing
            // insertion based on whether the target junction is already
            // occupied. Build a set of junction ids that currently have
            // vehicles in internal lanes; the per-vehicle check below will
            // skip candidates whose junction is occupied.
            let mut occupied_junctions: HashSet<u32> = HashSet::new();
            for (lane, lst) in &self.vehicles_by_lane {
                if let LaneId::Internal(jid, _) = lane {
                    if !lst.is_empty() {
                        occupied_junctions.insert(*jid);
                    }
                }
            }

            // Try to find a vehicle that can be inserted into an internal lane
            // for this junction. Iterate only over vehicles that are on roads
            // incoming to this junction using `vehicles_by_lane` to avoid
            // scanning the whole fleet.
            let mut inserted = false;
            // collect candidate vehicle indices from incoming normal lanes
            let mut candidates: HashSet<usize> = HashSet::new();
            for edge_ref in map.graph.edges_directed(node, petgraph::Direction::Incoming) {
                let eidx = edge_ref.id();
                let lanes = &edge_ref.weight().lanes;
                for (lid, _lane) in lanes.iter().enumerate() {
                    let lane_id = LaneId::Normal(eidx, lid as u32);
                    if let Some(lst) = self.vehicles_by_lane.get(&lane_id) {
                        for &vidx in lst {
                            candidates.insert(vidx);
                        }
                    }
                }
            }

            // iterate over candidate vehicles
            let mut cand_list: Vec<usize> = candidates.into_iter().collect();
            cand_list.sort_unstable();
            for vidx in cand_list {
                // Prepare a set of eligibility checks and print them for
                // debugging. We compute each boolean without early-continues
                // so the developer can see which condition fails.
                let v = &self.vehicles[vidx];

                let on_road = v.state == VehicleState::OnRoad;
                let current_lane = v.current_lane;
                let not_internal = !matches!(current_lane, Some(LaneId::Internal(_, _)));

                let from_edge_opt = match current_lane {
                    Some(LaneId::Normal(edge, _)) => Some(edge),
                    _ => None,
                };

                let has_entry = v.drive_plan.first().is_some();
                let entry_opt = v.drive_plan.first().cloned();
                // If the intended junction already contains vehicles, skip
                // attempting to force insertion for this vehicle.
                if let Some(ref e) = entry_opt {
                    if occupied_junctions.contains(&e.junction_id) {
                        continue;
                    }
                }
                let entry_junction_match = entry_opt.as_ref().map_or(false, |e| e.junction_id == junction_id);

                let lane_match = match (&entry_opt, from_edge_opt) {
                    (Some(e), Some(fe)) => matches!(e.lane_id, LaneId::Normal(ei, _) if ei == fe),
                    _ => false,
                };

                let link_opt = entry_opt.as_ref().and_then(|e| self.find_link(e.link_id));

                let ego_opt = entry_opt.as_ref().map(|e| ApproachData {
                    arrival_time: e.arrival_time,
                    leave_time: e.leave_time,
                    arrival_speed: e.arrival_speed,
                    leave_speed: e.leave_speed,
                    will_pass: true,
                });

                let link_open = match (&link_opt, &ego_opt) {
                    (Some(link), Some(ego)) => is_link_open(
                        &link,
                        &self.vehicles[vidx],
                        ego,
                        &self.link_states,
                        &self.vehicles_by_lane,
                        &self.vehicles,
                        junction_id,
                        LOOK_AHEAD,
                        STOP_DWELL_TIME,
                        &self.green_links,
                    ),
                    _ => false,
                };

                let road_len_opt = from_edge_opt.map(|fe| map.graph[fe].length);
                // Treat vehicles within 0.1m of the lane end as "at end" to
                // tolerate small numerical differences when approaching the
                // junction.
                let at_end = match road_len_opt {
                    Some(len) => self.vehicles[vidx].position_on_lane >= (len - 0.1),
                    None => false,
                };

                let entering_pos_opt = road_len_opt.map(|len| self.vehicles[vidx].position_on_lane - len);
                let to_lane_opt = entry_opt.as_ref().map(|e| LaneId::Internal(junction_id, e.via_internal_lane_id));
                let can_enter = match (to_lane_opt, entering_pos_opt) {
                    (Some(tl), Some(ent)) => self.can_enter_internal_lane(tl, ent, vidx),
                    _ => false,
                };

                if !on_road || !not_internal || !has_entry || !entry_junction_match || !lane_match || !link_open || !at_end || !can_enter {
                    continue;
                }

                // Safe to unwrap since we checked the options above.
                let entry = entry_opt.unwrap();
                let to_lane = to_lane_opt.unwrap();
                let entering_pos = entering_pos_opt.unwrap();

                // Force insertion: mimic `enter_junction_or_arrive`'s successful path.
                self.vehicles[vidx].position_on_lane = entering_pos;
                self.vehicles[vidx].current_lane = Some(to_lane);
                // remove the drive plan entry for this junction
                if !self.vehicles[vidx].drive_plan.is_empty() {
                    self.vehicles[vidx].drive_plan.remove(0);
                }

                lane_insert_sorted(&mut self.vehicles_by_lane, &self.vehicles, to_lane, vidx);

                // Register a pending transfer to remove the vehicle from its
                // previous lane and finalize the transfer in `flush_transfers`.
                self.pending_transfers.push(PendingTransfer {
                    vehicle_idx: vidx,
                    from_lane: entry.lane_id,
                    to_lane: Some(to_lane),
                });

                resolved_any = true;
                inserted = true;
                break;
            }

            if inserted {
                // reset the timer for this junction since we resolved it
                let node_mut = &mut self.config.map.graph[node];
                node_mut.interblocked_for = 0.0;
                // After resolving one insertion for this junction, move to next junction.
                continue;
            }
        }

        resolved_any
    }
}
