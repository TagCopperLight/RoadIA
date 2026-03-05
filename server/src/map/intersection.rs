use std::collections::HashMap;
use std::cmp::Ordering::{Less, Greater, Equal};

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
    pub fn new(id: u32, kind: IntersectionKind, name: String, x: f32, y: f32, intersection_type: IntersectionType) -> Self {
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

    fn compute_entry_exit_angles(&self, from: (f32, f32), to: (f32, f32)) -> (f32, f32) {
        let dx = self.x - from.0;
        let dy = self.y - from.1;
        let angle = dy.atan2(dx);
        let entry_angle = angle.to_degrees();

        let dx = to.0 - self.x;
        let dy = to.1 - self.y;
        let angle = dy.atan2(dx);
        let exit_angle = angle.to_degrees();
        
        (entry_angle, exit_angle)
    }

    pub fn set_rule(&mut self, road_id: u32, rule: IntersectionRules) {
        self.rules.insert(road_id, rule);
    }

    pub fn get_rule(&self, road_id: u32) -> IntersectionRules {
        match self.rules.get(&road_id) {
            Some(rule) => rule.clone(),
            None => panic!("Road {} not found in intersection {}", road_id, self.id)
        }
    }

    pub fn get_permission_to_enter(&self, vehicle_id: u64) -> bool {
        self.traffic_order.first() == Some(&vehicle_id)
    }

    pub fn request_intersection(&mut self, vehicle_id: u64, rule: IntersectionRules, arrival_time: f32, from: (f32, f32), to: (f32, f32)) {
        let (entry_angle, exit_angle) = self.compute_entry_exit_angles(from, to);
        let new_request = IntersectionRequest {
            vehicle_id,
            rule,
            entry_angle,
            exit_angle,
            arrival_time,
        };

        // 1. Check collisions with EXISTING requests (before adding the new one)
        let collisions = new_request.collisions_with(&self.requests, self.rules.len());

        // 2. Add to requests list exactly once
        self.requests.push(new_request.clone());

        if collisions.is_empty() {
            // 3. No conflicts: insert vehicle into traffic_order sorted by arrival time
            let insert_index = self.traffic_order
                .iter()
                .position(|&id| {
                    self.requests
                        .iter()
                        .find(|r| r.vehicle_id == id)
                        .map_or(false, |r| r.arrival_time > arrival_time)
                })
                .unwrap_or(self.traffic_order.len());
            self.traffic_order.insert(insert_index, vehicle_id);
        } else {
            // 4. Conflicts: include the new request in the priority group
            let mut all_conflicting = collisions;
            all_conflicting.push(new_request);
            let priority_order = IntersectionController::determine_priority(&all_conflicting);

            // Find the earliest position of any involved vehicle in traffic_order
            let mut insert_idx = self.traffic_order.len();
            for req in &priority_order {
                if let Some(pos) = self.traffic_order.iter().position(|&id| id == req.vehicle_id) {
                    if pos < insert_idx {
                        insert_idx = pos;
                    }
                }
            }

            self.traffic_order.retain(|id| !priority_order.iter().any(|req| req.vehicle_id == *id));

            let new_ids: Vec<u64> = priority_order.iter().map(|req| req.vehicle_id).collect();
            self.traffic_order.splice(insert_idx..insert_idx, new_ids);
        }
    }

    pub fn remove_request(&mut self, vehicle_id: u64) {
        self.requests.retain(|r| r.vehicle_id != vehicle_id);
        self.traffic_order.retain(|v| *v != vehicle_id);
    }
    
    fn paths_conflict(entry_angle_1: f32, exit_angle_1: f32, arrival_time_1: f32, entry_angle_2: f32, exit_angle_2: f32, arrival_time_2: f32, roads_count: usize) -> bool {
         let duration = 2.5;
         if (arrival_time_1 - arrival_time_2).abs() >= duration {
             return false;
         }

        if roads_count < 3 {
             return false;
        }

        let get_path_mask = |entry: f32, exit: f32, n: usize| -> u64 {
            let n_f = n as f32;
            let sector_width = 360.0 / n_f;
            
            // Normalize angles to [0, 360)
            let entry_norm = (entry % 360.0 + 360.0) % 360.0;
            let exit_norm = (exit % 360.0 + 360.0) % 360.0;

            let in_angle = (entry_norm + 180.0) % 360.0;
            let out_angle = exit_norm;

            let in_sector = ((in_angle + sector_width / 2.0) / sector_width).floor() as usize % n;
            let out_sector = ((out_angle + sector_width / 2.0) / sector_width).floor() as usize % n;
            
            let mut mask: u64 = 0;
            mask |= 1 << in_sector;
            mask |= 1 << out_sector;

            let diff = (out_sector + n - in_sector) % n;
            
            if diff != 1 {
                 mask |= 1 << n; // Center bit
            }
            
            mask
        };

        let mask1 = get_path_mask(entry_angle_1, exit_angle_1, roads_count);
        let mask2 = get_path_mask(entry_angle_2, exit_angle_2, roads_count);
        
        (mask1 & mask2) != 0
    }
}

impl IntersectionRequest {
    pub fn collisions_with(&self, others: &[IntersectionRequest], roads_count: usize) -> Vec<IntersectionRequest> {
        let mut collisions = Vec::new();
        for other in others {
            if self.vehicle_id == other.vehicle_id {
                continue;
            }
            if Intersection::paths_conflict(
                self.entry_angle, self.exit_angle, self.arrival_time,
                other.entry_angle, other.exit_angle, other.arrival_time,
                roads_count
            ) {
                collisions.push(other.clone());
            }
        }
        collisions
    }
}

impl IntersectionController {
    // Determine the priority order of the requests
    fn determine_priority(requests: &[IntersectionRequest]) -> Vec<IntersectionRequest> {
        // We know that all the requests are in conflict
        // That means that :
        // 1. If the request is from a Stop sign, it has already waited
        // 2. If the request is from a TrafficLight, it is green
        // The algorithm is :
        // 1. The order is separated in two, the ones that have priority and the ones that yield
        // 2. After this sort, we use the priority to the right for each group
        // 3. If the two requests have the same priority, we use the arrival time

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
                Less
            } else if delta < 180.0 && delta > 0.0 {
                Greater
            } else {
                a.arrival_time.partial_cmp(&b.arrival_time).unwrap_or(Equal)
            }
        };

        priority_requests.sort_by(sort_by_right_priority);
        yield_requests.sort_by(sort_by_right_priority);

        let mut result = Vec::new();
        for req in priority_requests {
            result.push(req.clone());
        }
        for req in yield_requests {
            result.push(req.clone());
        }
        result
    }
}