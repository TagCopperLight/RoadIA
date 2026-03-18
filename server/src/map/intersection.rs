use std::collections::HashMap;
use std::cmp::Ordering::{Equal, Greater, Less};

#[derive(Debug, Clone)]
pub enum IntersectionKind {
    Habitation,
    Intersection,
    Workplace,
}

#[derive(Clone, Debug, PartialEq)]
pub enum IntersectionRules {
    Yield,
    Priority,
    Stop,
    TrafficLight,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntersectionType {
    Priority,
    Stop,
    TrafficLight,
}

#[derive(Clone)]
pub struct Intersection {
    pub id: u32,
    pub kind: IntersectionKind,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub intersection_type: IntersectionType,

    pub rules: HashMap<u32, IntersectionRules>,
    pub requests: Vec<IntersectionRequest>,
    pub traffic_order: Vec<u64>,
}

#[derive(Clone)]
pub struct IntersectionRequest {
    pub vehicle_id: u64,
    pub rule: IntersectionRules,
    pub entry_angle: f32,
    pub exit_angle: f32,
    pub arrival_time: f32,
}

pub struct IntersectionController;

impl Intersection {
    pub fn new(
        id: u32,
        kind: IntersectionKind,
        name: String,
        x: f32,
        y: f32,
        intersection_type: IntersectionType,
    ) -> Self {
        Self {
            id,
            kind,
            name,
            x,
            y,
            intersection_type,
            rules: HashMap::new(),
            requests: Vec::new(),
            traffic_order: Vec::new(),
        }
    }

    pub fn set_rule(&mut self, road_id: u32, rule: IntersectionRules) {
        self.rules.insert(road_id, rule);
    }

    pub fn get_rule(&self, road_id: u32) -> IntersectionRules {
        match self.rules.get(&road_id) {
            Some(rule) => rule.clone(),
            None => panic!("Road {} not found in intersection {}", road_id, self.id),
        }
    }

    pub fn get_permission_to_enter(&self, vehicle_id: u64) -> bool {
        self.traffic_order.first() == Some(&vehicle_id)
    }

    pub fn request_intersection(
        &mut self,
        vehicle_id: u64,
        rule: IntersectionRules,
        arrival_time: f32,
        from: (f32, f32),
        to: (f32, f32),
    ) {
        let (entry_angle, exit_angle) = self.compute_entry_exit_angles(from, to);
        let new_request = IntersectionRequest { vehicle_id, rule, entry_angle, exit_angle, arrival_time };

        let collisions = new_request.collisions_with(&self.requests, self.rules.len());
        self.requests.push(new_request.clone());

        if collisions.is_empty() {
            self.insert_by_arrival_time(vehicle_id, arrival_time);
        } else {
            self.reorder_conflicting_group(collisions, new_request);
        }
    }

    pub fn remove_request(&mut self, vehicle_id: u64) {
        self.requests.retain(|r| r.vehicle_id != vehicle_id);
        self.traffic_order.retain(|v| *v != vehicle_id);
    }

    fn compute_entry_exit_angles(&self, from: (f32, f32), to: (f32, f32)) -> (f32, f32) {
        let entry_angle = {
            let dx = self.x - from.0;
            let dy = self.y - from.1;
            dy.atan2(dx).to_degrees()
        };
        let exit_angle = {
            let dx = to.0 - self.x;
            let dy = to.1 - self.y;
            dy.atan2(dx).to_degrees()
        };
        (entry_angle, exit_angle)
    }

    fn get_path_mask(entry: f32, exit: f32, n: usize) -> u64 {
        let sector_width = 360.0 / n as f32;

        let entry_norm = (entry % 360.0 + 360.0) % 360.0;
        let exit_norm = (exit % 360.0 + 360.0) % 360.0;

        let in_angle = (entry_norm + 180.0) % 360.0;
        let in_sector = ((in_angle + sector_width / 2.0) / sector_width).floor() as usize % n;
        let out_sector = ((exit_norm + sector_width / 2.0) / sector_width).floor() as usize % n;

        let mut mask: u64 = 0;
        mask |= 1 << in_sector;
        mask |= 1 << out_sector;

        let diff = (out_sector + n - in_sector) % n;
        if diff != 1 {
            mask |= 1 << n; // Center bit
        }

        mask
    }

    fn paths_conflict(
        entry_angle_1: f32, exit_angle_1: f32, arrival_time_1: f32,
        entry_angle_2: f32, exit_angle_2: f32, arrival_time_2: f32,
        roads_count: usize,
    ) -> bool {
        const CROSSING_DURATION: f32 = 2.5;

        if (arrival_time_1 - arrival_time_2).abs() >= CROSSING_DURATION {
            return false;
        }
        if roads_count < 3 {
            return false;
        }

        let mask1 = Self::get_path_mask(entry_angle_1, exit_angle_1, roads_count);
        let mask2 = Self::get_path_mask(entry_angle_2, exit_angle_2, roads_count);

        (mask1 & mask2) != 0
    }

    fn insert_by_arrival_time(&mut self, vehicle_id: u64, arrival_time: f32) {
        let insert_index = self.traffic_order
            .iter()
            .position(|&id| {
                self.requests
                    .iter()
                    .find(|r| r.vehicle_id == id)
                    .map_or(false, |r| r.arrival_time > arrival_time)
            })
            .unwrap_or(self.traffic_order.len());
        
        let insert_index = if self.traffic_order.is_empty() { insert_index } else { insert_index.max(1) };
        self.traffic_order.insert(insert_index, vehicle_id);
    }

    fn reorder_conflicting_group(
        &mut self,
        collisions: Vec<IntersectionRequest>,
        new_request: IntersectionRequest,
    ) {
        let all_conflicting: Vec<IntersectionRequest> = collisions.iter().cloned()
            .chain(std::iter::once(new_request.clone()))
            .collect();
        let priority_order = IntersectionController::determine_priority(&all_conflicting);

        let new_rank = priority_order
            .iter()
            .position(|r| r.vehicle_id == new_request.vehicle_id)
            .unwrap_or(priority_order.len());

        let insert_idx = self.traffic_order
            .iter()
            .enumerate()
            .filter_map(|(pos, &tid)| {
                let rank = priority_order.iter().position(|r| r.vehicle_id == tid)?;
                if rank < new_rank { Some(pos + 1) } else { None }
            })
            .max()
            .unwrap_or_else(|| {
                self.traffic_order
                    .iter()
                    .enumerate()
                    .find_map(|(pos, &tid)| {
                        let rank = priority_order.iter().position(|r| r.vehicle_id == tid)?;
                        if rank > new_rank { Some(pos) } else { None }
                    })
                    .unwrap_or(self.traffic_order.len())
            });

        let insert_idx = if self.traffic_order.is_empty() { insert_idx } else { insert_idx.max(1) };

        self.traffic_order.insert(insert_idx, new_request.vehicle_id);
    }
}

impl IntersectionRequest {
    pub fn collisions_with(&self, others: &[IntersectionRequest], roads_count: usize) -> Vec<IntersectionRequest> {
        others.iter()
            .filter(|other| {
                other.vehicle_id != self.vehicle_id
                    && Intersection::paths_conflict(
                        self.entry_angle, self.exit_angle, self.arrival_time,
                        other.entry_angle, other.exit_angle, other.arrival_time,
                        roads_count,
                    )
            })
            .cloned()
            .collect()
    }
}

impl IntersectionController {
    // Determine the priority order among conflicting requests.
    //
    // Assumptions when called:
    //   - Stop-sign vehicles have already waited.
    //   - TrafficLight vehicles are on green.
    //
    // Algorithm:
    //   1. Split into priority-road vs. yield-road groups.
    //   2. Within each group, apply right-of-way (vehicle on the right goes first).
    //   3. Ties broken by earliest arrival time.
    fn determine_priority(requests: &[IntersectionRequest]) -> Vec<IntersectionRequest> {
        let mut priority_requests: Vec<&IntersectionRequest> = Vec::new();
        let mut yield_requests: Vec<&IntersectionRequest> = Vec::new();

        for req in requests {
            match req.rule {
                IntersectionRules::Priority => priority_requests.push(req),
                _ => yield_requests.push(req),
            }
        }

        let sort_by_right_priority = |a: &&IntersectionRequest, b: &&IntersectionRequest| {
            let delta = (b.entry_angle - a.entry_angle + 360.0) % 360.0;
            if delta > 180.0 {
                Greater
            } else if delta < 180.0 && delta > 0.0 {
                Less
            } else {
                a.arrival_time.partial_cmp(&b.arrival_time).unwrap_or(Equal)
            }
        };

        priority_requests.sort_by(sort_by_right_priority);
        yield_requests.sort_by(sort_by_right_priority);

        priority_requests.into_iter()
            .chain(yield_requests)
            .map(|r| r.clone())
            .collect()
    }
}
