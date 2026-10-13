//! Drive plans: which junctions a vehicle is about to cross and when, and their
//! publication on the links for the right-of-way decisions of other vehicles.

use crate::map::road::LinkType;
use crate::simulation::config::{MIN_CREEP_SPEED, QUEUE_HEADWAY, STOP_DWELL_TIME};
use crate::simulation::kinematics;
use crate::simulation::vehicle::{DrivePlanEntry, LaneId, Vehicle, VehicleState};

use super::right_of_way::{Approach, ApproachData, QueueRank};
use super::SimulationEngine;

/// Seconds of driving at the current speed that a drive plan looks ahead, on top of the
/// braking distance and a fixed margin.
const PLAN_HORIZON_TIME: f32 = 3.0;
const PLAN_HORIZON_MARGIN: f32 = 50.0;

impl SimulationEngine {
    pub(super) fn plan_movements(&mut self) {
        for vidx in 0..self.vehicles.len() {
            if self.vehicles[vidx].state != VehicleState::OnRoad {
                continue;
            }
            let (mut plan, missing_turn) = self.build_drive_plan(vidx);
            if missing_turn && self.reroute_at_missing_turn(vidx) {
                plan = self.build_drive_plan(vidx).0;
            }
            self.vehicles[vidx].drive_plan = plan;
        }
    }

    /// Speed a vehicle is willing to drive at where the limit is `limit`.
    pub(super) fn speed_cap(vehicle: &Vehicle, limit: f32) -> f32 {
        if vehicle.is_reckless {
            vehicle.spec.max_speed
        } else {
            vehicle.spec.max_speed.min(limit)
        }
    }

    /// Lists the junctions within the planning horizon. The flag is set when the route
    /// asks for a turn that does not exist.
    pub(super) fn build_drive_plan(&self, vidx: usize) -> (Vec<DrivePlanEntry>, bool) {
        let map = &self.config.map;
        let v = &self.vehicles[vidx];
        let mut plan = Vec::new();
        let Some(lane) = v.current_lane else {
            return (plan, false);
        };

        // Distance from the vehicle's front to the next stop line, the lane it will
        // arrive on and the index in the path of the road leading to that junction.
        let (mut segment, mut in_lane, first_road) = match lane {
            LaneId::Normal(_, lane_id) => {
                (self.lane_length(lane) - v.position_on_lane, lane_id, v.path_index)
            }
            LaneId::Internal(..) => {
                let Some(info) = self.network.link_of_internal_lane(lane) else {
                    return (plan, false);
                };
                let LaneId::Normal(out_edge, out_lane) = info.out_lane else {
                    return (plan, false);
                };
                let out_len = map.graph[out_edge].length;
                (info.length - v.position_on_lane + out_len, out_lane, v.path_index + 1)
            }
        };

        let accel = v.spec.max_acceleration;
        let decel = v.spec.comfortable_deceleration;
        let horizon = kinematics::braking_distance(v.velocity, decel)
            + PLAN_HORIZON_TIME * v.velocity
            + PLAN_HORIZON_MARGIN;

        let mut distance = 0.0;
        let mut time = self.current_time;
        let mut speed = v.velocity;

        for i in first_road..v.path.len().saturating_sub(2) {
            distance += segment;
            if distance > horizon {
                break;
            }

            let in_edge = map.graph.find_edge(v.path[i], v.path[i + 1]);
            let out_edge = map.graph.find_edge(v.path[i + 1], v.path[i + 2]);
            let (Some(in_edge), Some(out_edge)) = (in_edge, out_edge) else {
                return (plan, true);
            };
            let Some(info) = self.network.choose_link(map, in_edge, in_lane, out_edge) else {
                return (plan, true);
            };

            let road_in = &map.graph[in_edge];
            let road_out = &map.graph[out_edge];
            let approach_speed = kinematics::approach_speed(
                &info.link.link_type,
                Self::speed_cap(v, road_in.speed_limit),
            )
            .min(Self::speed_cap(v, info.speed_limit));
            let leave_speed = Self::speed_cap(v, info.speed_limit.min(road_out.speed_limit));

            let (to_line, arrival_speed) =
                kinematics::arrival(segment, speed, approach_speed, accel, decel);
            let mut arrival_time = time + to_line;
            if info.link.link_type == LinkType::Stop && !v.is_reckless {
                let waited = if i == first_road { v.waiting_time } else { 0.0 };
                arrival_time += (STOP_DWELL_TIME - waited).max(0.0);
            }
            let leave_time = kinematics::leave_time(
                arrival_time,
                info.length,
                v.spec.length,
                arrival_speed,
                leave_speed,
                accel,
            );

            plan.push(DrivePlanEntry {
                link_id: info.link.id,
                junction_id: info.junction_id,
                via_internal_lane_id: info.link.via_internal_lane_id,
                distance,
                approach_speed: approach_speed.max(MIN_CREEP_SPEED),
                arrival_time,
                arrival_speed,
                leave_time,
                leave_speed,
            });

            time = arrival_time;
            speed = arrival_speed;
            segment = info.length + road_out.length;
            in_lane = match info.out_lane {
                LaneId::Normal(_, lane_id) => lane_id,
                LaneId::Internal(..) => 0,
            };
        }

        (plan, false)
    }

    /// Publishes every drive plan on the links it crosses. Lanes are walked front to back
    /// so that each vehicle knows its leader: a follower never crosses before its leader,
    /// and a vehicle queued behind a waiting one shares that vehicle's queue precedence.
    pub(super) fn register_approaches(&mut self) {
        for lane in self.occupied_lanes() {
            let list = self.vehicles_by_lane[&lane].clone();
            let mut leader: Option<(f32, Option<QueueRank>)> = None;

            for &vidx in list.iter().rev() {
                if self.vehicles[vidx].state != VehicleState::OnRoad {
                    continue;
                }

                let vehicle = &mut self.vehicles[vidx];
                let own_rank = (vehicle.waiting_time > 0.0).then_some(QueueRank {
                    waited: vehicle.waiting_time,
                    head: vidx,
                });
                let rank = own_rank.or(leader.and_then(|(_, rank)| rank));

                if let (Some((leader_arrival, _)), Some(first)) = (leader, vehicle.drive_plan.first()) {
                    let delay = leader_arrival + QUEUE_HEADWAY - first.arrival_time;
                    if delay > 0.0 {
                        for entry in &mut vehicle.drive_plan {
                            entry.arrival_time += delay;
                            entry.leave_time += delay;
                        }
                    }
                }

                leader = vehicle.drive_plan.first().map(|entry| (entry.arrival_time, rank));
                self.queue_ranks[vidx] = rank;
                self.register(vidx);
            }
        }
    }

    fn register(&mut self, vidx: usize) {
        self.unregister(vidx);
        let vehicle = &mut self.vehicles[vidx];
        for (k, entry) in vehicle.drive_plan.iter().enumerate() {
            let Some(approaches) = self.approaches.get_mut(entry.link_id as usize) else {
                continue;
            };
            approaches.push(Approach {
                vehicle: vidx,
                data: ApproachData { arrival_time: entry.arrival_time, leave_time: entry.leave_time },
                queue: if k == 0 { self.queue_ranks[vidx] } else { None },
            });
            vehicle.registered_link_ids.push(entry.link_id);
        }
    }

    pub(super) fn unregister(&mut self, vidx: usize) {
        for link_id in std::mem::take(&mut self.vehicles[vidx].registered_link_ids) {
            if let Some(approaches) = self.approaches.get_mut(link_id as usize) {
                if let Some(at) = approaches.iter().position(|a| a.vehicle == vidx) {
                    approaches.swap_remove(at);
                }
            }
        }
    }
}
