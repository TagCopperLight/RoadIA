//! LC2013-style lane changes: overtake a slow leader on the left, keep right otherwise.
//! Lanes are numbered from the centre of the road: lane 0 is the leftmost one.

use rustc_hash::FxHashMap;

use crate::simulation::config::{
    LC2013_COOLDOWN, LC2013_MAX_DECEL_FOLLOWER, LC2013_SAFE_GAP_FRONT, LC2013_SAFE_GAP_REAR,
    LC2013_SPEED_GAIN_THRESHOLD,
};
use crate::simulation::vehicle::{LaneId, VehicleState};

use super::SimulationEngine;

struct LaneChange {
    vidx: usize,
    from: LaneId,
    to: LaneId,
}

impl SimulationEngine {
    pub(super) fn attempt_lane_changes(&mut self) {
        let mut changes: Vec<LaneChange> = Vec::new();

        for vidx in 0..self.vehicles.len() {
            let vehicle = &self.vehicles[vidx];
            let Some(from @ LaneId::Normal(edge, lane_id)) = vehicle.current_lane else {
                continue;
            };
            if vehicle.state != VehicleState::OnRoad || vehicle.lane_change_cooldown > 0.0 {
                continue;
            }
            let road = &self.config.map.graph[edge];
            let lane_count = road.lanes.len() as u32;
            if lane_count < 2 {
                continue;
            }

            let next_road_id = vehicle
                .drive_plan
                .first()
                .and_then(|entry| self.network.link(entry.link_id))
                .map(|info| info.link.destination_road_id);
            let leads_to_next_road = |target: u32| {
                next_road_id.is_none_or(|road_id| {
                    road.lanes
                        .iter()
                        .find(|lane| lane.id == target)
                        .is_some_and(|lane| lane.links.iter().any(|l| l.destination_road_id == road_id))
                })
            };
            let pos = vehicle.position_on_lane;
            let length = vehicle.spec.length;
            let fits = |target: LaneId| {
                let (front_gap, front_speed) = self.vehicle_ahead_in_lane(target, pos);
                let (rear_gap, rear_speed) = self.vehicle_behind_in_lane(target, pos, length);
                let follower_safe = rear_speed < 0.01
                    || rear_speed * rear_speed / (2.0 * rear_gap.max(0.01)) <= LC2013_MAX_DECEL_FOLLOWER;
                let safe = front_gap >= LC2013_SAFE_GAP_FRONT
                    && rear_gap >= LC2013_SAFE_GAP_REAR
                    && follower_safe;
                (safe, front_gap, front_speed)
            };

            let (current_gap, current_speed) = match self.leader_on_lane(vidx) {
                Some(leader) => {
                    let lv = &self.vehicles[leader];
                    (lv.position_on_lane - lv.spec.length - pos, lv.previous_velocity)
                }
                None => (f32::INFINITY, 0.0),
            };
            let desired = vehicle.spec.max_speed.min(road.speed_limit);

            // Overtake: blocked by a slow leader and the left lane is better.
            if lane_id > 0 && leads_to_next_road(lane_id - 1) {
                let to = LaneId::Normal(edge, lane_id - 1);
                let (safe, gap, speed) = fits(to);
                let blocked = current_gap < 2.0 * LC2013_SAFE_GAP_FRONT
                    && desired > current_speed + LC2013_SPEED_GAIN_THRESHOLD;
                let better = gap > 2.0 * LC2013_SAFE_GAP_FRONT
                    || speed > current_speed + LC2013_SPEED_GAIN_THRESHOLD;
                if safe && blocked && better {
                    changes.push(LaneChange { vidx, from, to });
                    continue;
                }
            }

            // Keep right when it does not slow the vehicle down.
            if lane_id + 1 < lane_count && leads_to_next_road(lane_id + 1) {
                let to = LaneId::Normal(edge, lane_id + 1);
                let (safe, gap, speed) = fits(to);
                let no_slower = speed >= vehicle.velocity - LC2013_SPEED_GAIN_THRESHOLD
                    || gap > 2.0 * LC2013_SAFE_GAP_FRONT;
                if safe && no_slower {
                    changes.push(LaneChange { vidx, from, to });
                }
            }
        }

        self.apply_lane_changes(changes);
    }

    fn apply_lane_changes(&mut self, changes: Vec<LaneChange>) {
        // Two vehicles side by side swapping lanes would drive through each other.
        let mut by_move: FxHashMap<(LaneId, LaneId), Vec<f32>> = FxHashMap::default();
        for change in &changes {
            by_move
                .entry((change.from, change.to))
                .or_default()
                .push(self.vehicles[change.vidx].position_on_lane);
        }
        let swaps = |change: &LaneChange| {
            let pos = self.vehicles[change.vidx].position_on_lane;
            by_move.get(&(change.to, change.from)).is_some_and(|positions| {
                positions.iter().any(|other| (pos - other).abs() < LC2013_SAFE_GAP_FRONT)
            })
        };
        let changes: Vec<LaneChange> = changes.into_iter().filter(|change| !swaps(change)).collect();

        for LaneChange { vidx, from, to } in changes {
            if self.vehicles[vidx].current_lane != Some(from) {
                continue;
            }
            // Gaps may have changed with the moves already applied.
            let pos = self.vehicles[vidx].position_on_lane;
            let length = self.vehicles[vidx].spec.length;
            let (front_gap, _) = self.vehicle_ahead_in_lane(to, pos);
            let (rear_gap, _) = self.vehicle_behind_in_lane(to, pos, length);
            if front_gap < LC2013_SAFE_GAP_FRONT || rear_gap < LC2013_SAFE_GAP_REAR {
                continue;
            }

            self.move_vehicle_to_lane(vidx, to);
            let plan = self.build_drive_plan(vidx).0;
            let vehicle = &mut self.vehicles[vidx];
            vehicle.drive_plan = plan;
            vehicle.lane_change_cooldown = LC2013_COOLDOWN;
            vehicle.waiting_time = 0.0;
            vehicle.impatience = 0.0;
        }
    }
}
