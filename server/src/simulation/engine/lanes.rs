//! Lane occupancy: sorted vehicle lists per lane and neighbour queries.

use crate::simulation::vehicle::LaneId;

use super::SimulationEngine;

impl SimulationEngine {
    pub(super) fn rebuild_lane_slots(&mut self) {
        self.lane_slots.iter_mut().for_each(|slot| *slot = None);
        for list in self.vehicles_by_lane.values() {
            for (slot, &vidx) in list.iter().enumerate() {
                self.lane_slots[vidx] = Some(slot);
            }
        }
    }

    pub(crate) fn insert_vehicle_into_lane(&mut self, lane: LaneId, vidx: usize) {
        let position = self.vehicles[vidx].position_on_lane;
        let list = self.vehicles_by_lane.entry(lane).or_default();
        let insert_at =
            list.partition_point(|&other| self.vehicles[other].position_on_lane < position);
        list.insert(insert_at, vidx);
        for (slot, &other) in list.iter().enumerate().skip(insert_at) {
            self.lane_slots[other] = Some(slot);
        }
    }

    pub(crate) fn remove_vehicle_from_lane(&mut self, lane: LaneId, vidx: usize) {
        self.lane_slots[vidx] = None;
        let Some(list) = self.vehicles_by_lane.get_mut(&lane) else {
            return;
        };
        let Some(removed_at) = list.iter().position(|&other| other == vidx) else {
            return;
        };
        list.remove(removed_at);
        if list.is_empty() {
            self.vehicles_by_lane.remove(&lane);
            return;
        }
        for (slot, &other) in list.iter().enumerate().skip(removed_at) {
            self.lane_slots[other] = Some(slot);
        }
    }

    pub(super) fn move_vehicle_to_lane(&mut self, vidx: usize, to: LaneId) {
        if let Some(from) = self.vehicles[vidx].current_lane {
            self.remove_vehicle_from_lane(from, vidx);
        }
        self.vehicles[vidx].current_lane = Some(to);
        self.insert_vehicle_into_lane(to, vidx);
    }

    /// The vehicle directly ahead on the same lane.
    pub(super) fn leader_on_lane(&self, vidx: usize) -> Option<usize> {
        let lane = self.vehicles[vidx].current_lane?;
        let slot = self.lane_slots[vidx]?;
        self.vehicles_by_lane.get(&lane)?.get(slot + 1).copied()
    }

    /// The rearmost vehicle of a lane.
    pub(super) fn rear_of_lane(&self, lane: LaneId) -> Option<usize> {
        self.vehicles_by_lane.get(&lane)?.first().copied()
    }

    /// Gap and speed of the first vehicle whose front is at or beyond `pos` on `lane`.
    /// The gap is negative when that vehicle overlaps `pos`.
    pub(super) fn vehicle_ahead_in_lane(&self, lane: LaneId, pos: f32) -> (f32, f32) {
        let Some(list) = self.vehicles_by_lane.get(&lane) else {
            return (f32::INFINITY, 0.0);
        };
        let split = list.partition_point(|&i| self.vehicles[i].position_on_lane < pos);
        match list.get(split) {
            Some(&leader) => {
                let lv = &self.vehicles[leader];
                (lv.position_on_lane - lv.spec.length - pos, lv.previous_velocity)
            }
            None => (f32::INFINITY, 0.0),
        }
    }

    /// Gap and speed of the last vehicle whose front is behind `pos` on `lane`, for a
    /// vehicle of length `length` whose front is at `pos`.
    pub(super) fn vehicle_behind_in_lane(&self, lane: LaneId, pos: f32, length: f32) -> (f32, f32) {
        let Some(list) = self.vehicles_by_lane.get(&lane) else {
            return (f32::INFINITY, 0.0);
        };
        let split = list.partition_point(|&i| self.vehicles[i].position_on_lane < pos);
        match split.checked_sub(1).map(|i| list[i]) {
            Some(follower) => {
                let fv = &self.vehicles[follower];
                (pos - length - fv.position_on_lane, fv.previous_velocity)
            }
            None => (f32::INFINITY, 0.0),
        }
    }

    pub(super) fn lane_length(&self, lane: LaneId) -> f32 {
        self.network.lane_length(&self.config.map, lane)
    }
}
