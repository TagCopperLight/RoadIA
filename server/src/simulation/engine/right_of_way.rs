//! Right of way: may a vehicle cross a link now?
//!
//! Every vehicle decides for itself by looking at the drive plans its foes registered on
//! conflicting links. Vehicles that are waiting at their stop line are ordered by how long
//! they have waited: when two waiting vehicles conflict, the one that waited longest goes
//! first, so a ring of vehicles yielding to each other cannot block forever.

use crate::map::road::LinkType;
use crate::simulation::config::{
    CONFLICT_TIME_GAP, EMERGENCY_DECEL_FACTOR, EXIT_SPACE_LOOKAHEAD, STOPPED_SPEED, STOP_DWELL_TIME,
};
use crate::simulation::kinematics;

use super::network::LinkInfo;
use super::signals::Signal;
use super::SimulationEngine;

/// When a vehicle expects to occupy a junction lane.
#[derive(Clone, Debug)]
pub struct ApproachData {
    pub arrival_time: f32,
    pub leave_time: f32,
}

#[derive(Clone, Debug)]
pub(crate) struct Approach {
    pub vehicle: usize,
    pub data: ApproachData,
    /// Set when this is the vehicle's next junction and its queue is waiting there.
    pub queue: Option<QueueRank>,
}

/// Precedence of a queue waiting at a stop line: its head and how long the head waited.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct QueueRank {
    pub waited: f32,
    /// Index of the vehicle at the head of the queue.
    pub head: usize,
}

impl QueueRank {
    pub fn goes_before(self, other: QueueRank) -> bool {
        self.waited > other.waited || (self.waited == other.waited && self.head < other.head)
    }
}

/// Why a vehicle may not cross a link.
#[derive(Clone, Debug, PartialEq)]
pub enum Blocker {
    Signal,
    StopSign,
    /// A vehicle on a conflicting path is inside the junction.
    JunctionBusy { vehicle_id: u64 },
    /// The lane after the junction has no room for the vehicle.
    ExitFull,
    /// A vehicle with right of way is coming.
    Foe { vehicle_id: u64 },
    UnknownLink,
}

/// Whether two vehicles would be in the junction at the same time, with a safety `gap`.
pub fn time_windows_conflict(ego: &ApproachData, foe: &ApproachData, gap: f32) -> bool {
    ego.arrival_time < foe.leave_time + gap && foe.arrival_time < ego.leave_time + gap
}

impl SimulationEngine {
    /// Why vehicle `vidx` may not cross the next junction on its plan, if it may not.
    /// Diagnostics helper.
    pub fn blocking_reason(&self, vidx: usize) -> Option<Blocker> {
        if self.vehicles[vidx].drive_plan.is_empty() {
            return None;
        }
        self.check_link(vidx, 0).err()
    }

    /// Whether vehicle `vidx` may cross the `k`-th link of its drive plan.
    pub(super) fn link_open(&self, vidx: usize, k: usize) -> bool {
        self.check_link(vidx, k).is_ok()
    }

    fn check_link(&self, vidx: usize, k: usize) -> Result<(), Blocker> {
        let vehicle = &self.vehicles[vidx];
        let entry = &vehicle.drive_plan[k];
        let Some(info) = self.network.link(entry.link_id) else {
            return Err(Blocker::UnknownLink);
        };

        if !vehicle.is_reckless {
            let decel = vehicle.spec.comfortable_deceleration;
            let can_stop_with =
                |decel: f32| kinematics::braking_distance(vehicle.velocity, decel) <= entry.distance;
            match self.signals.of(entry.link_id) {
                Signal::Red if can_stop_with(decel * EMERGENCY_DECEL_FACTOR) => return Err(Blocker::Signal),
                Signal::Yellow if can_stop_with(decel) => return Err(Blocker::Signal),
                _ => {}
            }
            if info.link.link_type == LinkType::Stop
                && !(k == 0 && vehicle.waiting_time >= STOP_DWELL_TIME)
            {
                return Err(Blocker::StopSign);
            }
        }

        let foes = || info.link.foe_links.iter().filter_map(|foe| self.network.link(foe.id));

        if let Some(occupant) = foes().find_map(|foe| self.junction_lane_busy(foe, entry.arrival_time)) {
            return Err(Blocker::JunctionBusy { vehicle_id: self.vehicles[occupant].id });
        }
        if info.keep_clear && !self.exit_has_room(vidx, info) {
            return Err(Blocker::ExitFull);
        }

        let ego = ApproachData { arrival_time: entry.arrival_time, leave_time: entry.leave_time };
        let ego_rank = if k == 0 { self.queue_ranks[vidx] } else { None };
        // Impatient drivers accept shorter gaps.
        let time_gap = CONFLICT_TIME_GAP * (1.0 - 0.5 * vehicle.impatience);

        for foe in foes() {
            if self.signals.of(foe.link.id) == Signal::Red || !info.must_yield_to(foe) {
                continue;
            }
            // Between equals, the queue that waited longest goes first.
            let first_come_first_served = info.rank() == foe.rank();

            for approach in &self.approaches[foe.link.id as usize] {
                if approach.vehicle == vidx {
                    continue;
                }
                let foe_rank = approach.queue;
                if foe_rank.is_some_and(|rank| self.queue_stuck(rank)) {
                    continue;
                }

                let blocks = match (ego_rank, foe_rank) {
                    (Some(ego_rank), Some(foe_rank)) => {
                        !(first_come_first_served && ego_rank.goes_before(foe_rank))
                    }
                    _ => time_windows_conflict(&ego, &approach.data, time_gap),
                };
                if blocks {
                    return Err(Blocker::Foe { vehicle_id: self.vehicles[approach.vehicle].id });
                }
            }
        }

        Ok(())
    }

    /// Whether a waiting queue cannot move because its head has nowhere to go.
    fn queue_stuck(&self, rank: QueueRank) -> bool {
        self.vehicles[rank.head]
            .drive_plan
            .first()
            .and_then(|entry| self.network.link(entry.link_id))
            .is_some_and(|link| link.keep_clear && !self.exit_has_room(rank.head, link))
    }

    /// A vehicle inside the junction on `lane_of` that will still be there when a vehicle
    /// arriving at `arrival_time` enters.
    fn junction_lane_busy(&self, lane_of: &LinkInfo, arrival_time: f32) -> Option<usize> {
        let list = self.vehicles_by_lane.get(&lane_of.internal_lane)?;
        list.iter().copied().find(|&vidx| {
            let occupant = &self.vehicles[vidx];
            let remaining = lane_of.length - occupant.position_on_lane + occupant.spec.length;
            let clear_time = self.current_time + remaining / occupant.velocity.max(STOPPED_SPEED);
            arrival_time < clear_time + CONFLICT_TIME_GAP
        })
    }

    /// Whether the lane after the junction has room for the vehicle, counting the vehicles
    /// already inside the junction that are heading there. Entering without room would
    /// leave the vehicle stuck in the junction, blocking every crossing flow.
    pub(super) fn exit_has_room(&self, vidx: usize, link: &LinkInfo) -> bool {
        let claimed = self.exit_claims.get(&link.out_lane).copied().unwrap_or(0.0);
        let free = match self.rear_of_lane(link.out_lane) {
            Some(rear) => {
                let rv = &self.vehicles[rear];
                let moving_away = if rv.velocity > 2.0 * STOPPED_SPEED {
                    rv.velocity * EXIT_SPACE_LOOKAHEAD
                } else {
                    0.0
                };
                rv.position_on_lane - rv.spec.length + moving_away
            }
            // An empty lane always takes a vehicle, even when shorter than it.
            None if claimed == 0.0 => return true,
            None => self.lane_length(link.out_lane),
        };
        let needed = self.vehicles[vidx].spec.length + self.config.minimum_gap;
        free - claimed >= needed
    }
}
