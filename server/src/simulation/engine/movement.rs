//! Moving vehicles: link decisions, car following and lane transitions.

use crate::simulation::config::{IMPATIENCE_RATE, STOPPED_SPEED, STOP_LINE_TOLERANCE};
use crate::simulation::kinematics;
use crate::simulation::vehicle::{LaneId, VehicleState};

use super::SimulationEngine;

/// Extra distance (m), beyond the braking distance, over which a vehicle checks links.
const DECISION_MARGIN: f32 = 20.0;
/// Extra distance (m), beyond the braking distance, over which a vehicle looks for a leader.
const LEADER_MARGIN: f32 = 30.0;
/// Bound on the lanes a vehicle can go through in one step (very short lanes).
const MAX_LANES_PER_STEP: usize = 16;

/// What a vehicle may do at the junctions ahead during this step.
struct Decision {
    /// Number of drive plan entries, from the first, the vehicle may cross.
    open_links: usize,
    /// Distance to the stop line the vehicle must stop at, if any.
    stop_at: Option<f32>,
    /// Speed allowing to slow down to the approach speed of the open links.
    speed_cap: f32,
}

impl SimulationEngine {
    pub(super) fn execute_movements(&mut self) {
        // Fixed order, lane by lane from the back: every vehicle moves exactly once even
        // though lane lists change while vehicles move between lanes.
        let order: Vec<usize> = self
            .occupied_lanes()
            .iter()
            .flat_map(|lane| self.vehicles_by_lane[lane].iter().copied())
            .collect();

        for vidx in order {
            if self.vehicles[vidx].state == VehicleState::OnRoad {
                self.drive(vidx);
            }
        }
    }

    fn drive(&mut self, vidx: usize) {
        let Some(lane) = self.vehicles[vidx].current_lane else {
            return;
        };
        let decision = self.decide(vidx);
        let (mut gap, mut leader_speed) = self.leader_gap(vidx, decision.open_links);
        if let Some(stop) = decision.stop_at {
            if stop < gap {
                gap = stop.max(0.01);
                leader_speed = 0.0;
            }
        }

        let vehicle = &self.vehicles[vidx];
        let limit = Self::speed_cap(
            vehicle,
            self.network.lane_speed_limit(&self.config.map, lane),
        );
        let accel = vehicle.compute_acceleration(
            limit.min(decision.speed_cap),
            self.config.minimum_gap,
            gap,
            leader_speed,
        );

        let dt = self.config.time_step;
        let vehicle = &mut self.vehicles[vidx];
        vehicle.velocity = (vehicle.velocity + accel * dt).max(0.0);
        let travelled = vehicle.velocity * dt;
        vehicle.position_on_lane += travelled;
        vehicle.distance_traveled += travelled;

        self.advance(vidx, decision.open_links);
        self.update_waiting(vidx);
    }

    fn decide(&self, vidx: usize) -> Decision {
        let vehicle = &self.vehicles[vidx];
        let decel = vehicle.spec.comfortable_deceleration;
        let lookahead = kinematics::braking_distance(vehicle.velocity, decel)
            + 2.0 * vehicle.velocity
            + self.config.minimum_gap
            + DECISION_MARGIN;

        let mut decision = Decision { open_links: 0, stop_at: None, speed_cap: f32::INFINITY };
        for (k, entry) in vehicle.drive_plan.iter().enumerate() {
            if entry.distance > lookahead {
                break;
            }
            if !self.link_open(vidx, k) {
                decision.stop_at = Some(entry.distance);
                return decision;
            }
            decision.speed_cap = decision
                .speed_cap
                .min(kinematics::v_approach(entry.distance, entry.approach_speed, decel));
            decision.open_links += 1;
        }

        // Without a plan (the route asks for a missing turn) the vehicle waits at the line.
        if vehicle.drive_plan.is_empty() && !self.on_final_road(vidx) {
            if let Some(lane @ LaneId::Normal(..)) = vehicle.current_lane {
                decision.stop_at = Some(self.lane_length(lane) - vehicle.position_on_lane);
            }
        }
        decision
    }

    /// Gap to, and speed of, the vehicle ahead, looking through the junctions the vehicle
    /// is allowed to cross.
    fn leader_gap(&self, vidx: usize, open_links: usize) -> (f32, f32) {
        let vehicle = &self.vehicles[vidx];
        if let Some(leader) = self.leader_on_lane(vidx) {
            let lv = &self.vehicles[leader];
            let gap = lv.position_on_lane - lv.spec.length - vehicle.position_on_lane;
            return (gap.max(0.01), lv.previous_velocity);
        }
        let Some(lane) = vehicle.current_lane else {
            return (f32::INFINITY, 0.0);
        };

        let mut distance = self.lane_length(lane) - vehicle.position_on_lane;
        let mut best = (f32::INFINITY, 0.0);

        // The previous vehicle of this lane may be crossing the junction on another path.
        if let Some(&previous) = self.last_into_junction.get(&lane) {
            let pv = &self.vehicles[previous];
            if previous != vidx
                && pv.state == VehicleState::OnRoad
                && matches!(pv.current_lane, Some(LaneId::Internal(..)))
            {
                best = (distance + pv.position_on_lane - pv.spec.length, pv.previous_velocity);
            }
        }

        let horizon = kinematics::braking_distance(vehicle.velocity, vehicle.spec.comfortable_deceleration)
            + 2.0 * vehicle.velocity
            + LEADER_MARGIN;

        let current_exit = match lane {
            LaneId::Internal(..) => self.network.link_of_internal_lane(lane).map(|info| info.out_lane),
            LaneId::Normal(..) => None,
        };
        let crossings = vehicle.drive_plan[..open_links]
            .iter()
            .filter_map(|entry| self.network.link(entry.link_id))
            .flat_map(|info| [info.internal_lane, info.out_lane]);

        for next in current_exit.into_iter().chain(crossings) {
            if distance > horizon || distance >= best.0 {
                break;
            }
            if let Some(rear) = self.rear_of_lane(next).filter(|&rear| rear != vidx) {
                let rv = &self.vehicles[rear];
                let gap = distance + rv.position_on_lane - rv.spec.length;
                if gap < best.0 {
                    best = (gap, rv.previous_velocity);
                }
                break;
            }
            distance += self.lane_length(next);
        }

        (best.0.max(0.01), best.1)
    }

    /// Moves the vehicle onto the next lanes once it has driven past the end of its lane.
    fn advance(&mut self, vidx: usize, mut permits: usize) {
        for _ in 0..MAX_LANES_PER_STEP {
            let vehicle = &self.vehicles[vidx];
            if vehicle.state != VehicleState::OnRoad {
                return;
            }
            let Some(lane) = vehicle.current_lane else {
                return;
            };
            let length = self.lane_length(lane);
            if vehicle.position_on_lane < length {
                return;
            }

            match lane {
                LaneId::Normal(..) if self.on_final_road(vidx) => {
                    self.finish_trip(vidx);
                    return;
                }
                LaneId::Normal(..) => {
                    if permits == 0 || !self.enter_junction(vidx, length) {
                        // Not allowed in: stop at the line.
                        let vehicle = &mut self.vehicles[vidx];
                        vehicle.position_on_lane = (length - 0.01).max(0.0);
                        vehicle.velocity = 0.0;
                        return;
                    }
                    permits -= 1;
                }
                LaneId::Internal(..) => self.leave_junction(vidx, length),
            }
        }
    }

    pub(super) fn on_final_road(&self, vidx: usize) -> bool {
        let vehicle = &self.vehicles[vidx];
        matches!(vehicle.current_lane, Some(LaneId::Normal(..)))
            && vehicle.path_index + 2 >= vehicle.path.len()
    }

    fn enter_junction(&mut self, vidx: usize, lane_length: f32) -> bool {
        let Some(entry) = self.vehicles[vidx].drive_plan.first() else {
            return false;
        };
        let planned = entry.link_id;
        let link_id = self.maybe_navigation_mistake(vidx, planned).unwrap_or(planned);
        let Some(info) = self.network.link(link_id) else {
            return false;
        };
        let (internal_lane, out_lane) = (info.internal_lane, info.out_lane);
        let from = self.vehicles[vidx].current_lane;

        let claim = self.vehicles[vidx].spec.length + self.config.minimum_gap;
        *self.exit_claims.entry(out_lane).or_default() += claim;
        if let Some(from) = from {
            self.last_into_junction.insert(from, vidx);
        }

        let vehicle = &mut self.vehicles[vidx];
        vehicle.drive_plan.remove(0);
        vehicle.position_on_lane -= lane_length;
        vehicle.waiting_time = 0.0;
        vehicle.impatience = 0.0;
        if let Some(&junction) = vehicle.path.get(vehicle.path_index + 1) {
            vehicle.waypoints.retain(|&waypoint| waypoint != junction);
        }

        self.move_vehicle_to_lane(vidx, internal_lane);
        true
    }

    fn leave_junction(&mut self, vidx: usize, lane_length: f32) {
        let Some(lane) = self.vehicles[vidx].current_lane else {
            return;
        };
        let Some(out_lane) = self.network.link_of_internal_lane(lane).map(|info| info.out_lane) else {
            return;
        };

        let claim = self.vehicles[vidx].spec.length + self.config.minimum_gap;
        if let Some(claimed) = self.exit_claims.get_mut(&out_lane) {
            *claimed -= claim;
            if *claimed <= 1e-3 {
                self.exit_claims.remove(&out_lane);
            }
        }

        let vehicle = &mut self.vehicles[vidx];
        vehicle.position_on_lane -= lane_length;
        vehicle.path_index += 1;
        self.move_vehicle_to_lane(vidx, out_lane);
    }

    /// Counts the time spent standing at a stop line, which orders waiting vehicles.
    fn update_waiting(&mut self, vidx: usize) {
        let vehicle = &self.vehicles[vidx];
        let at_line = match vehicle.current_lane {
            Some(lane @ LaneId::Normal(..)) if vehicle.state == VehicleState::OnRoad => {
                !vehicle.drive_plan.is_empty()
                    && self.lane_length(lane) - vehicle.position_on_lane
                        <= self.config.minimum_gap + STOP_LINE_TOLERANCE
            }
            _ => false,
        };

        let dt = self.config.time_step;
        let vehicle = &mut self.vehicles[vidx];
        if at_line && vehicle.velocity < STOPPED_SPEED {
            vehicle.waiting_time += dt;
            vehicle.impatience = (vehicle.waiting_time * IMPATIENCE_RATE).min(1.0);
        }
    }
}
