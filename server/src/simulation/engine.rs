use crate::map::intersection::IntersectionKind;
use crate::map::model::Map;
use crate::simulation::config::SimulationConfig;
use crate::simulation::handle::Handle;
use crate::simulation::metrics::SimulationMetrics;
use crate::simulation::vehicle::{RoutingStrategy, Vehicle, VehicleState};
use petgraph::graph::NodeIndex;

pub struct Engine<R: RoutingStrategy> {
    pub map: Map,
    pub vehicles: Vec<Vehicle>,
    pub config: SimulationConfig,
    pub routing: R,
    pub metrics: SimulationMetrics,
    pub current_time_s: f32,
    pub handle: Handle,
    pub crossing_intersection_until: Option<f32>,
}

impl<R: RoutingStrategy> Engine<R> {
    pub fn new(
        map: Map,
        vehicles: Vec<Vehicle>,
        config: SimulationConfig,
        routing: R,
        handle: Handle,
    ) -> Self {
        println!("[SIM INIT] Creating simulation…");

        for a in &vehicles {
            println!(
                "[SIM INIT] Vehicle {} initial pos BEFORE start_trip = ({}, {})",
                a.id, a.x, a.y
            );
        }

        Self {
            map,
            vehicles,
            config: config.clone(),
            routing,
            metrics: SimulationMetrics::default(),
            current_time_s: config.start_time_s,
            handle,
            crossing_intersection_until: None,
        }
    }

    pub fn run(&mut self) {
        println!("[SIM] Running simulation…");

        while self.current_time_s <= self.config.end_time_s {
            self.step();
            self.current_time_s += self.config.time_step_s;
        }
    }

    fn step(&mut self) {
        let dt = self.config.time_step_s;

        println!("\n--- STEP t = {:.2} ---", self.current_time_s);

        for i in 0..self.vehicles.len() {
            let a = &self.vehicles[i];
            println!(
                "[VEHICLE {}] state={:?} pos=({:.2},{:.2}) edge_pos={:.2}",
                a.id, a.state, a.x, a.y, a.position_on_edge_m
            );

            match a.state {
                VehicleState::WaitingToDepart => {
                    println!("[VEHICLE {}] WaitingToDepart", a.id);
                    if self.current_time_s >= a.trip.departure_time_s as f32 {
                        println!("[VEHICLE {}] DEPART NOW", a.id);
                        self.start_trip(i);
                    }
                }
                VehicleState::EnRoute => {
                    self.update_en_route(i, dt);
                }
                VehicleState::AtIntersection => {
                    self.update_at_intersection(i, dt);
                }
                VehicleState::Arrived => {
                    println!("[VEHICLE {}] Arrived", a.id);
                }
            }
        }

        self.handle.update_vehicles(self.vehicles.clone());
    }

    fn start_trip(&mut self, i: usize) {
        let vehicle = &mut self.vehicles[i];

        println!("[START_TRIP] Vehicle {} starting trip", vehicle.id);

        let origin_idx = self.map.index_from_id(vehicle.trip.origin_id as u32);
        let dest_idx = self.map.index_from_id(vehicle.trip.destination_id as u32);

        let path = self.routing.compute_path(
            &self.map,
            origin_idx,
            dest_idx,
            vehicle.trip.departure_time_s,
        );

        println!("[START_TRIP] Path = {:?}", path);

        vehicle.current_node = path[0];
        vehicle.next_node = Some(path[1]);
        vehicle.path = path;
        vehicle.path_index = 0;
        vehicle.position_on_edge_m = 0.0;

        let node = &self.map.graph[vehicle.current_node];
        vehicle.x = node.x;
        vehicle.y = node.y;

        println!(
            "[START_TRIP] Vehicle {} pos RESET to node {:?} = ({}, {})",
            vehicle.id, vehicle.current_node, vehicle.x, vehicle.y
        );

        vehicle.state = VehicleState::EnRoute;
    }

    fn update_en_route(&mut self, i: usize, dt: f32) {
        let vehicle = &mut self.vehicles[i];

        let next = match vehicle.next_node {
            Some(n) => n,
            None => {
                println!("[ERROR] Vehicle {} has no next_node!", vehicle.id);
                return;
            }
        };

        let current = vehicle.current_node;

        let edge = self
            .map
            .graph
            .edges_connecting(current, next)
            .next()
            .expect("No edge between nodes");

        let segment = edge.weight();

        let speed_m_s = vehicle.spec.max_speed_kmh * 1000.0 / 3600.0;

        println!(
            "[MOVE] Vehicle {} speed={} m/s dt={}s",
            vehicle.id, speed_m_s, dt
        );

        vehicle.position_on_edge_m += speed_m_s * dt;

        println!(
            "[MOVE] Vehicle {} edge_pos={:.2} / {}",
            vehicle.id, vehicle.position_on_edge_m, segment.length_m
        );

        if vehicle.position_on_edge_m >= segment.length_m {
            println!(
                "[NODE ARRIVAL] Vehicle {} reached node {:?}",
                vehicle.id, next
            );

            vehicle.position_on_edge_m = 0.0;
            vehicle.current_node = next;
            vehicle.path_index += 1;

            if vehicle.path_index + 1 >= vehicle.path.len() {
                println!("[ARRIVED] Vehicle {} reached destination", vehicle.id);
                vehicle.state = VehicleState::Arrived;
                return;
            }

            // Check if we arrived at an intersection
            let next_node_kind = self.map.graph[vehicle.current_node].kind.clone();
            // Assuming IntersectionKind is imported and matches
            if matches!(next_node_kind, IntersectionKind::Intersection) {
                println!(
                    "[INTERSECTION] Vehicle {} arrived at intersection {:?}",
                    vehicle.id, vehicle.current_node
                );
                // Release the borrow before calling handle_intersection_arrival
                self.handle_intersection_arrival(i);
                return; // Don't continue with normal movement
            } else {
                vehicle.next_node = Some(vehicle.path[vehicle.path_index + 1]);
            }
        }

        let n1 = &self.map.graph[current];
        let n2 = &self.map.graph[next];

        let t = vehicle.position_on_edge_m / segment.length_m;

        vehicle.x = n1.x + t * (n2.x - n1.x);
        vehicle.y = n1.y + t * (n2.y - n1.y);

        println!(
            "[MOVE] Vehicle {} new pos = ({:.2}, {:.2}) t={:.3}",
            vehicle.id, vehicle.x, vehicle.y, t
        );
    }

    fn handle_intersection_arrival(&mut self, vehicle_index: usize) {
        let current_intersection = self.vehicles[vehicle_index].current_node;

        // Find the next node in the path (destination)
        let target_node = if self.vehicles[vehicle_index].path_index + 1
            < self.vehicles[vehicle_index].path.len()
        {
            self.vehicles[vehicle_index].path[self.vehicles[vehicle_index].path_index + 1]
        } else {
            println!(
                "[ERROR] Vehicle {} has no next node in path",
                self.vehicles[vehicle_index].id
            );
            return;
        };

        println!(
            "[INTERSECTION] Vehicle {} at intersection {:?} heading to {:?}",
            self.vehicles[vehicle_index].id, current_intersection, target_node
        );

        // Check if intersection is occupied or if there's a crossing timer
        let intersection_occupied = if let Some(until_time) = self.crossing_intersection_until {
            self.current_time_s < until_time
        } else {
            false
        };

        // Check if any other vehicle is approaching or at this intersection
        let other_vehicle_at_intersection = self.vehicles.iter().any(|other| {
            other.id != self.vehicles[vehicle_index].id
                && (other.state == VehicleState::AtIntersection
                    || (other.next_node == Some(current_intersection)
                        && other.state != VehicleState::Arrived))
        });

        if intersection_occupied || other_vehicle_at_intersection {
            // Intersection is occupied, must wait
            println!(
                "[INTERSECTION] Vehicle {} must wait (intersection occupied)",
                self.vehicles[vehicle_index].id
            );
            self.vehicles[vehicle_index].state = VehicleState::AtIntersection;
            self.vehicles[vehicle_index].next_node = Some(target_node);
            self.vehicles[vehicle_index].intersection_wait_start_time_s = Some(self.current_time_s);
            self.adjust_waiting_position(vehicle_index, current_intersection);
        } else {
            // Intersection is free, can proceed without stopping
            println!(
                "[INTERSECTION] Vehicle {} can proceed freely (no conflict)",
                self.vehicles[vehicle_index].id
            );
            self.vehicles[vehicle_index].next_node = Some(target_node);
            // Mark intersection as crossing for a brief moment to prevent simultaneous crossing
            self.crossing_intersection_until = Some(self.current_time_s + 0.5);
        }
    }

    fn check_right_of_way(&self, vehicle_index: usize, intersection: NodeIndex) -> bool {
        let vehicle = &self.vehicles[vehicle_index];

        // Check if intersection is currently being crossed (has crossing timer)
        if let Some(until_time) = self.crossing_intersection_until {
            if self.current_time_s < until_time {
                return false;
            }
        }

        // Find all vehicles that are either approaching or already at this intersection
        let mut conflicting_vehicles = Vec::new();

        for other_vehicle in &self.vehicles {
            if other_vehicle.id == vehicle.id {
                continue;
            }

            // Check if other vehicle is approaching this intersection or already waiting at it
            if let Some(next_node) = other_vehicle.next_node {
                if next_node == intersection {
                    // This vehicle is either approaching or waiting at the intersection
                    conflicting_vehicles.push(other_vehicle);
                }
            }
        }

        // Priority rule: car coming from the right has priority
        // H1 comes from the right, H2 from the left, so H1 has priority over H2
        if vehicle.id == 0 {
            // H1 (right side)
            // H1 has priority, always proceed
            return true;
        }
        if vehicle.id == 1 {
            // H2 (left side)
            // H2 waits for H1 if H1 is approaching or at intersection
            for other_vehicle in &conflicting_vehicles {
                if other_vehicle.id == 0 {
                    // H1 has priority, H2 must wait
                    return false;
                }
            }
        }
        // Other cars can proceed
        true
    }

    fn adjust_waiting_position(&mut self, vehicle_index: usize, intersection: NodeIndex) {
        let vehicle = &mut self.vehicles[vehicle_index];
        let intersection_pos = &self.map.graph[intersection];

        // Calculate approach direction
        let previous_node = if vehicle.path_index > 0 {
            vehicle.path[vehicle.path_index - 1]
        } else {
            return; // No previous node
        };

        let prev_pos = &self.map.graph[previous_node];
        let dx = intersection_pos.x - prev_pos.x;
        let dy = intersection_pos.y - prev_pos.y;

        // Normalize direction
        let length = (dx * dx + dy * dy).sqrt();
        if length > 0.0 {
            let nx = dx / length;
            let ny = dy / length;

            // Position exactly at the intersection (no offset)
            let offset = 0.0;
            vehicle.x = intersection_pos.x - nx * offset;
            vehicle.y = intersection_pos.y - ny * offset;
        }
    }

    fn update_at_intersection(&mut self, vehicle_index: usize, _dt: f32) {
        let current_intersection = self.vehicles[vehicle_index].current_node;

        println!(
            "[INTERSECTION] Vehicle {} waiting at intersection {:?}",
            self.vehicles[vehicle_index].id, current_intersection
        );

        // Check if waiting time has elapsed (3 seconds for better visualization)
        let wait_duration = 3.0; // 3 seconds delay
        if let Some(wait_start) = self.vehicles[vehicle_index].intersection_wait_start_time_s {
            if self.current_time_s - wait_start < wait_duration {
                println!(
                    "[INTERSECTION] Vehicle {} still waiting (time: {:.1}s / {:.1}s)",
                    self.vehicles[vehicle_index].id,
                    self.current_time_s - wait_start,
                    wait_duration
                );
                return; // Continue waiting
            }
        }

        // Check again for right of way
        if self.check_right_of_way(vehicle_index, current_intersection) {
            println!(
                "[INTERSECTION] Vehicle {} now has right of way, proceeding",
                self.vehicles[vehicle_index].id
            );
            self.vehicles[vehicle_index].state = VehicleState::EnRoute;
            self.vehicles[vehicle_index].intersection_wait_start_time_s = None; // Reset timer
                                                                                // Mark intersection as being crossed for 2 seconds
            self.crossing_intersection_until = Some(self.current_time_s + 2.0);
        } else {
            println!(
                "[INTERSECTION] Vehicle {} still waiting",
                self.vehicles[vehicle_index].id
            );
            // Reset waiting timer if still waiting
            self.vehicles[vehicle_index].intersection_wait_start_time_s = Some(self.current_time_s);
        }
    }
}
