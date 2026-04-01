use std::collections::HashMap;

use crate::scoring;
use crate::simulation::config::{
    SimulationConfig, IMPATIENCE_RATE, LOOK_AHEAD, MIN_CREEP_SPEED, STOP_DWELL_TIME,
};
use crate::map::intersection::{ApproachData, LinkState, is_link_open};
use crate::simulation::kinematics;
use crate::simulation::vehicle::{DrivePlanEntry, LaneId, Vehicle, VehicleState};
use petgraph::graph::{EdgeIndex, NodeIndex};

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

pub struct SimulationEngine {
    pub config: SimulationConfig,
    pub vehicles: Vec<Vehicle>,
    pub current_time: f32,
    pub vehicles_by_lane: HashMap<LaneId, Vec<usize>>, // Sorted by position_on_lane (back → front = index 0 first).
    pub link_states: HashMap<u32, LinkState>,
    pending_transfers: Vec<PendingTransfer>,
}

impl Simulation for SimulationEngine {
    fn new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self {
        let current_time = config.start_time;
        Self {
            config,
            vehicles,
            current_time,
            vehicles_by_lane: HashMap::new(),
            link_states: HashMap::new(),
            pending_transfers: Vec::new(),
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
    }
}

// Departures
impl SimulationEngine {
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
            let vlen = self.vehicles[vidx].spec.length;

            let space_ok = self
                .vehicles_by_lane
                .get(&lane_id)
                .and_then(|lst| lst.first().copied())
                .is_none_or(|rear_idx| {
                    self.vehicles[rear_idx].position_on_lane - self.vehicles[rear_idx].spec.length
                        >= vlen
                });

            if !space_ok {
                continue;
            }

            self.vehicles[vidx].position_on_lane = vlen;
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

            if v.velocity < 0.1 && !v.drive_plan.is_empty() {
                v.waiting_time += dt;
                v.impatience = (v.waiting_time * IMPATIENCE_RATE).min(1.0);
            } else if v.velocity > 0.5 {
                v.waiting_time = 0.0;
                v.impatience = 0.0;
            }
        }

        // After applying IDM (velocity/position update), update speed gain probability,
        // attempt a strategic overtake, then fall back to instant lane change if needed.
        self.update_speed_gain_probability(vidx, dt);
        let _overtake = self.try_strategical_overtake(vidx);
        let _switched = if !_overtake { self.try_switch_line(vidx) } else { false };

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

    fn lane_length_by_id(&self, lane: LaneId) -> Option<f32> {
        match lane {
            LaneId::Normal(edge, _) => Some(self.config.map.graph[edge].length),
            LaneId::Internal(jid, ilid) => self
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
                }),
        }
    }

    fn lane_occupancy_from(&self, lane: LaneId, from_distance: f32) -> Option<f32> {
        let lane_len = self.lane_length_by_id(lane)?;

        if from_distance >= lane_len {
            return Some(0.0);
        }

        let section_len = lane_len - from_distance;
        let mut occupied = 0.0f32;

        if let Some(list) = self.vehicles_by_lane.get(&lane) {
            for &vidx in list {
                let v = &self.vehicles[vidx];
                let v_start = (v.position_on_lane - v.spec.length).max(0.0);
                let v_end = v.position_on_lane.min(lane_len);

                let overlap_start = v_start.max(from_distance);
                let overlap_end = v_end;
                let overlap = (overlap_end - overlap_start).max(0.0);
                occupied += overlap;
            }
        }

        Some((occupied / section_len).clamp(0.0, 1.0))
    }

    pub fn lane_mean_speed_from(&self, lane: LaneId, from_distance: f32) -> f32 {
        let lane_len = self.lane_length_by_id(lane).unwrap_or(1.0);
        let lane_speed_limit = match lane {
            LaneId::Normal(edge, _) => self.config.map.graph[edge].speed_limit,
            LaneId::Internal(jid, ilid) => self
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
        };

        if from_distance >= lane_len {
            return lane_speed_limit;
        }

        let mut weighted_speed = 0.0f32;
        let mut total_overlap = 0.0f32;

        if let Some(list) = self.vehicles_by_lane.get(&lane) {
            for &vidx in list {
                let v = &self.vehicles[vidx];
                let v_start = (v.position_on_lane - v.spec.length).max(0.0);
                let v_end = v.position_on_lane.min(lane_len);

                let overlap_start = v_start.max(from_distance);
                let overlap_end = v_end;
                let overlap = (overlap_end - overlap_start).max(0.0);
                if overlap > 0.0 {
                    weighted_speed += overlap * v.velocity;
                    total_overlap += overlap;
                }
            }
        }

        if total_overlap > 0.0 {
            (weighted_speed / total_overlap).max(0.0)
        } else {
            lane_speed_limit
        }
    }

    pub fn lane_changing_urgency(&self, vidx: usize, lookahead_speed: f32) -> bool {
        let v = match self.vehicles.get(vidx) {
            Some(v) => v,
            None => return false,
        };

        if v.is_on_correct_lane(&self.config.map) {
            return false;
        }

        let best_offset = match v.lane_index_offset_to_correct_lane(&self.config.map) {
            Some(o) => o,
            None => return false,
        };

        if best_offset == 0 {
            return false;
        }

        let (edge_idx, lane_id, position_on_lane) = match v.current_lane {
            Some(LaneId::Normal(e, lid)) => (e, lid, v.position_on_lane),
            _ => return false,
        };

        let d = (self.lane_length(vidx) - position_on_lane).max(0.0);

        let road = &self.config.map.graph[edge_idx];
        let current_idx = match road.lanes.iter().position(|l| l.id == lane_id) {
            Some(i) => i as isize,
            None => return false,
        };

        let target_idx_isize = current_idx + best_offset as isize;
        if target_idx_isize < 0 || target_idx_isize >= road.lanes.len() as isize {
            return false;
        }
        let target_idx = target_idx_isize as usize;
        let target_lane = LaneId::Normal(edge_idx, road.lanes[target_idx].id);
        let o = match self.lane_occupancy_from(target_lane, position_on_lane) {
            Some(x) => x,
            None => return false,
        };
        
        let f = if best_offset > 0 {20.0} else {10.0};

        d * o < lookahead_speed * (best_offset.abs() as f32) * f
    }

    /// Return true if the target `lane` has no vehicle occupying the interval [start, start+length].
    /// Vehicles are assumed to occupy [position_on_lane - spec.length, position_on_lane].
    pub fn lane_has_gap_between(&self, lane: LaneId, start: f32, length: f32) -> bool {
        let lane_len = match self.lane_length_by_id(lane) {
            Some(l) => l,
            None => return false,
        };

        if start >= lane_len {
            return false;
        }

        let end = (start + length).min(lane_len);

        if let Some(list) = self.vehicles_by_lane.get(&lane) {
            for &vidx in list {
                let v = &self.vehicles[vidx];
                let v_start = (v.position_on_lane - v.spec.length).max(0.0);
                let v_end = v.position_on_lane.min(lane_len);

                // If vehicle interval overlaps [start, end) there is no gap
                if v_end > start && v_start < end {
                    return false;
                }
            }
        }

        true
    }

    /// Try to switch lane for vehicle `vidx` based on `speed_gain_probability`.
    /// - If `speed_gain_probability > 0.2` try left; if `< -2.0` try right.
    /// - Uses `lane_has_gap_between` to check the availability of the interval
    ///   corresponding to the vehicle rear..front ([pos - len, pos]).
    /// Returns true if the switch was performed.
    pub fn try_switch_line(&mut self, vidx: usize) -> bool {
        // gather read-only data first
        let (edge_idx, lane_id, pos, veh_len, speed_prob) = match self.vehicles.get(vidx) {
            Some(v) => match v.current_lane {
                Some(LaneId::Normal(e, lid)) => (e, lid, v.position_on_lane, v.spec.length, v.speed_gain_probability),
                _ => return false,
            },
            None => return false,
        };

        let road = &self.config.map.graph[edge_idx];
        let current_idx = match road.lanes.iter().position(|l| l.id == lane_id) {
            Some(i) => i,
            None => return false,
        };

        let mut target_idx_opt: Option<usize> = None;
        if speed_prob > 0.2 && current_idx > 0 {
            target_idx_opt = Some(current_idx - 1);
        } else if speed_prob < -2.0 && current_idx + 1 < road.lanes.len() {
            target_idx_opt = Some(current_idx + 1);
        }

        let target_idx = match target_idx_opt {
            Some(i) => i,
            None => return false,
        };

        let target_lane = LaneId::Normal(edge_idx, road.lanes[target_idx].id);
        let start = (pos - veh_len).max(0.0);

        if !self.lane_has_gap_between(target_lane, start, veh_len) {
            return false;
        }

        // perform lane change: remove from old lane list, update vehicle, insert into new lane list
        let current_lane = LaneId::Normal(edge_idx, lane_id);

        if let Some(lst) = self.vehicles_by_lane.get_mut(&current_lane) {
            lst.retain(|&i| i != vidx);
        }

        // update vehicle lane
        if let Some(v) = self.vehicles.get_mut(vidx) {
            v.current_lane = Some(target_lane);
            v.speed_gain_probability = 0.0;
        }

        // insert in target lane sorted by position
        lane_insert_sorted(&mut self.vehicles_by_lane, &self.vehicles, target_lane, vidx);

        true
    }

    pub fn try_strategical_overtake(&mut self, vidx: usize) -> bool {
        let (edge_idx, lane_id, pos, veh_len) = match self.vehicles.get(vidx) {
            Some(v) => match v.current_lane {
                Some(LaneId::Normal(e, lid)) => (e, lid, v.position_on_lane, v.spec.length),
                _ => return false,
            },
            None => return false,
        };

        let road = &self.config.map.graph[edge_idx];
        let current_idx = match road.lanes.iter().position(|l| l.id == lane_id) {
            Some(i) => i,
            None => return false,
        };

        let offset = match self.vehicles.get(vidx).and_then(|v| v.lane_index_offset_to_correct_lane(&self.config.map)) {
            Some(o) if o != 0 => o,
            _ => return false,
        };

        let target_idx_opt: Option<usize> = if offset > 0 {
            // need to move to higher index (right)
            if current_idx + 1 < road.lanes.len() {
                Some(current_idx + 1)
            } else {
                None
            }
        } else {
            // need to move to lower index (left)
            if current_idx > 0 {
                Some(current_idx - 1)
            } else {
                None
            }
        };

        let target_idx = match target_idx_opt {
            Some(i) => i,
            None => return false,
        };

        let target_lane = LaneId::Normal(edge_idx, road.lanes[target_idx].id);
        let start = (pos - veh_len).max(0.0);

        if !self.lane_has_gap_between(target_lane, start, veh_len) {
            return false;
        }

        // perform lane change: remove from old lane list, update vehicle, insert into new lane list
        let current_lane = LaneId::Normal(edge_idx, lane_id);

        if let Some(lst) = self.vehicles_by_lane.get_mut(&current_lane) {
            lst.retain(|&i| i != vidx);
        }

        if let Some(v) = self.vehicles.get_mut(vidx) {
            v.current_lane = Some(target_lane);
            v.speed_gain_probability = 0.0;
        }

        lane_insert_sorted(&mut self.vehicles_by_lane, &self.vehicles, target_lane, vidx);

        true
    }

    pub fn update_speed_gain_probability(&mut self, vidx: usize, time_step: f32) {
        let (edge_idx, lane_id, pos, ego_speed, ego_max_speed) = match self.vehicles.get(vidx) {
            Some(v) => match v.current_lane {
                Some(LaneId::Normal(e, lid)) => (e, lid, v.position_on_lane, v.velocity.max(0.1), v.spec.max_speed),
                _ => return,
            },
            None => return,
        };

        let road = &self.config.map.graph[edge_idx];
        let current_idx = match road.lanes.iter().position(|l| l.id == lane_id) {
            Some(i) => i,
            None => return,
        };

        let mut total_gain = 0.0f32;
        let mut faster_lane = false;
        let mean_lane = self.lane_mean_speed_from(LaneId::Normal(edge_idx, road.lanes[current_idx].id), pos+0.01);

        if current_idx > 0 {
            let left_lane = LaneId::Normal(edge_idx, road.lanes[current_idx - 1].id);
            let consider_left = {
                if let Some(v) = self.vehicles.get(vidx) {
                    let mut tmp = v.clone();
                    tmp.current_lane = Some(left_lane);
                    tmp.is_on_correct_lane(&self.config.map)
                } else {
                    false
                }
            };

            if consider_left {
                let mean_left = self.lane_mean_speed_from(left_lane, pos);
                let left_gain_factor = time_step * (mean_left.min(ego_max_speed) - mean_lane) / mean_lane;
                if left_gain_factor > 0.1 {
                    faster_lane = true;
                    total_gain += left_gain_factor;
                }
            }
        }

        if current_idx + 1 < road.lanes.len() {
            let right_lane = LaneId::Normal(edge_idx, road.lanes[current_idx + 1].id);
            // Only consider right lane if it is a 'correct' lane towards the vehicle's next node
            let consider_right = {
                if let Some(v) = self.vehicles.get(vidx) {
                    let mut tmp = v.clone();
                    tmp.current_lane = Some(right_lane);
                    tmp.is_on_correct_lane(&self.config.map)
                } else {
                    false
                }
            };

            if consider_right {
                let mean_right = self.lane_mean_speed_from(right_lane, pos);
                let right_gain_factor = time_step * (mean_right.min(ego_max_speed) - mean_lane) / mean_lane;
                if right_gain_factor > 0.1 {
                    faster_lane = true;
                    total_gain -= right_gain_factor;
                }
            }
        }


        if let Some(v) = self.vehicles.get_mut(vidx) {
            v.speed_gain_probability += total_gain;
            if !faster_lane{
                v.speed_gain_probability = v.speed_gain_probability * (0.5_f32).powf(time_step);
            }
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
        if pi + 1 >= self.vehicles[vidx].path.len() {
            self.vehicles[vidx].state = VehicleState::Arrived;
            self.pending_transfers.push(PendingTransfer { vehicle_idx: vidx, from_lane, to_lane: None });
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
        self.vehicles[vidx].current_lane = Some(to_lane);
        self.pending_transfers.push(PendingTransfer { vehicle_idx: vidx, from_lane, to_lane: Some(to_lane) });
    }

    fn enter_junction_or_arrive(&mut self, vidx: usize, from_lane: LaneId, in_edge: EdgeIndex) {
        let road_len = self.config.map.graph[in_edge].length;
        let pi = self.vehicles[vidx].path_index;
        let path_len = self.vehicles[vidx].path.len();

        if pi + 1 >= path_len - 1 {
            self.vehicles[vidx].position_on_lane = road_len;
            self.vehicles[vidx].state = VehicleState::Arrived;
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

        self.vehicles[vidx].current_lane = Some(to_lane);
        self.vehicles[vidx].drive_plan.remove(0);
        self.pending_transfers.push(PendingTransfer { vehicle_idx: vidx, from_lane, to_lane: Some(to_lane) });
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
                    lane_insert_sorted(&mut self.vehicles_by_lane, &self.vehicles, to_lane, t.vehicle_idx);
                }
            }
        }
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
