use std::collections::HashSet;

use petgraph::graph::NodeIndex;
use serde::Deserialize;

use crate::map::model::Map;
use crate::simulation::config::MAX_DURATION;
use crate::simulation::vehicle::{fastest_path, TripRequest, Vehicle, VehicleKind, VehicleSpec, VehicleState};

#[derive(Clone, Debug, Deserialize)]
pub struct ShiftProfileInput {
    pub origin: u32,
    pub destination: u32,
    pub departure_time: f32,
    pub dwell_time: f32,
}

struct ShiftProfileState {
    input: ShiftProfileInput,
    origin_node: NodeIndex,
    destination_node: NodeIndex,
    return_spawned: bool,
}

pub struct ShiftScheduler {
    profiles: Vec<ShiftProfileState>,
    next_vehicle_id: u64,
}

impl ShiftScheduler {
    pub fn new(map: &Map, shift_profiles: Vec<ShiftProfileInput>) -> Result<Self, String> {
        let mut seen_profiles: HashSet<(u32, u32, u32)> = HashSet::new();
        let mut profiles = Vec::with_capacity(shift_profiles.len());

        for input in shift_profiles {
            if !input.departure_time.is_finite() || input.departure_time < 0.0 {
                return Err(format!("Invalid departure_time for shift {} -> {}", input.origin, input.destination));
            }
            if !input.dwell_time.is_finite() || input.dwell_time < 0.0 {
                return Err(format!("Invalid dwell_time for shift {} -> {}", input.origin, input.destination));
            }
            if input.departure_time > MAX_DURATION {
                return Err(format!(
                    "departure_time {} exceeds MAX_DURATION for shift {} -> {}",
                    input.departure_time, input.origin, input.destination
                ));
            }

            let key = (input.origin, input.destination, input.departure_time.to_bits());
            if !seen_profiles.insert(key) {
                return Err(format!(
                    "Duplicate shift profile for {} -> {} at departure_time {}",
                    input.origin, input.destination, input.departure_time
                ));
            }

            if input.origin == input.destination {
                return Err(format!(
                    "Shift profile must use two distinct nodes ({} -> {})",
                    input.origin, input.destination
                ));
            }

            let origin_node = map
                .find_node(input.origin)
                .ok_or_else(|| format!("Origin node {} not found", input.origin))?;
            let destination_node = map
                .find_node(input.destination)
                .ok_or_else(|| format!("Destination node {} not found", input.destination))?;

            if fastest_path(map, origin_node, destination_node).is_none() {
                return Err(format!(
                    "No outbound path found for shift {} -> {}",
                    input.origin, input.destination
                ));
            }
            if fastest_path(map, destination_node, origin_node).is_none() {
                return Err(format!(
                    "No return path found for shift {} -> {}",
                    input.destination, input.origin
                ));
            }

            profiles.push(ShiftProfileState {
                input,
                origin_node,
                destination_node,
                return_spawned: false,
            });
        }

        Ok(Self {
            profiles,
            next_vehicle_id: 0,
        })
    }

    pub fn build_initial_vehicles(&mut self) -> Vec<Vehicle> {
        let mut vehicles = Vec::with_capacity(self.profiles.len());

        for index in 0..self.profiles.len() {
            let (origin_node, destination_node, departure_time) = {
                let profile = &self.profiles[index];
                (
                    profile.origin_node,
                    profile.destination_node,
                    profile.input.departure_time,
                )
            };
            let vehicle_id = self.allocate_vehicle_id();
            vehicles.push(Self::build_vehicle(
                vehicle_id,
                origin_node,
                destination_node,
                departure_time,
            ));
        }

        vehicles
    }

    pub fn spawn_due_return_vehicles(
        &mut self,
        current_time: f32,
        vehicles: &[Vehicle],
        map: &Map,
    ) -> Result<Vec<Vehicle>, String> {
        let mut spawned = Vec::new();
        let mut due_profiles = Vec::new();

        for index in 0..self.profiles.len() {
            let profile = &self.profiles[index];
            if profile.return_spawned {
                continue;
            }

            let Some(outbound_vehicle) = vehicles.iter().find(|vehicle| {
                vehicle.trip.origin == profile.origin_node
                    && vehicle.trip.destination == profile.destination_node
                    && vehicle.trip.departure_time.to_bits() == profile.input.departure_time.to_bits()
                    && matches!(vehicle.state, VehicleState::Arrived)
            }) else {
                continue;
            };

            let Some(arrived_at) = outbound_vehicle.arrived_at else {
                continue;
            };

            let return_departure_time = arrived_at + profile.input.dwell_time;
            if current_time < return_departure_time {
                continue;
            }

            due_profiles.push((
                index,
                profile.destination_node,
                profile.origin_node,
                return_departure_time,
                profile.input.destination,
                profile.input.origin,
            ));
        }

        for (index, origin_node, destination_node, return_departure_time, destination_id, origin_id) in due_profiles {
            let vehicle_id = self.allocate_vehicle_id();
            let mut return_vehicle = Self::build_vehicle(
                vehicle_id,
                origin_node,
                destination_node,
                return_departure_time,
            );
            return_vehicle.update_path(map);
            if return_vehicle.path.len() < 2 {
                return Err(format!(
                    "No path found for return trip {} -> {}",
                    destination_id, origin_id
                ));
            }

            self.profiles[index].return_spawned = true;
            spawned.push(return_vehicle);
        }

        Ok(spawned)
    }

    pub fn has_pending_returns(&self) -> bool {
        self.profiles.iter().any(|profile| !profile.return_spawned)
    }

    fn allocate_vehicle_id(&mut self) -> u64 {
        let vehicle_id = self.next_vehicle_id;
        self.next_vehicle_id += 1;
        vehicle_id
    }

    fn build_vehicle(
        vehicle_id: u64,
        origin: NodeIndex,
        destination: NodeIndex,
        departure_time: f32,
    ) -> Vehicle {
        Vehicle::new(
            vehicle_id,
            default_vehicle_spec(),
            TripRequest {
                origin,
                destination,
                departure_time,
            },
        )
    }
}

fn default_vehicle_spec() -> VehicleSpec {
    VehicleSpec::new(VehicleKind::Car, 40.0, 4.0, 3.0, 1.0, 10.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test::make_minimal_straight_map;
    use crate::simulation::vehicle::VehicleState;

    #[test]
    fn scheduler_builds_initial_vehicles() {
        let map = make_minimal_straight_map();
        let scheduler = ShiftScheduler::new(
            &map,
            vec![ShiftProfileInput {
                origin: 0,
                destination: 2,
                departure_time: 12.0,
                dwell_time: 30.0,
            }],
        )
        .expect("scheduler should build");

        let mut scheduler = scheduler;
        let vehicles = scheduler.build_initial_vehicles();
        assert_eq!(vehicles.len(), 1);
        assert_eq!(vehicles[0].trip.departure_time, 12.0);
        assert_eq!(vehicles[0].trip.origin, map.find_node(0).unwrap());
        assert_eq!(vehicles[0].trip.destination, map.find_node(2).unwrap());
    }

    #[test]
    fn scheduler_spawns_return_vehicle_once() {
        let map = make_minimal_straight_map();
        let mut scheduler = ShiftScheduler::new(
            &map,
            vec![ShiftProfileInput {
                origin: 0,
                destination: 2,
                departure_time: 0.0,
                dwell_time: 2.0,
            }],
        )
        .expect("scheduler should build");

        let mut vehicles = scheduler.build_initial_vehicles();
        let mut outbound = vehicles.pop().unwrap();
        outbound.state = VehicleState::Arrived;
        outbound.arrived_at = Some(5.0);
        vehicles.push(outbound);

        let spawned = scheduler
            .spawn_due_return_vehicles(7.0, &vehicles, &map)
            .expect("return vehicle should spawn");
        assert_eq!(spawned.len(), 1);
        assert_eq!(spawned[0].trip.departure_time, 7.0);
        assert_eq!(spawned[0].trip.origin, map.find_node(2).unwrap());
        assert_eq!(spawned[0].trip.destination, map.find_node(0).unwrap());
        assert!(scheduler.has_pending_returns() == false);

        let spawned_again = scheduler
            .spawn_due_return_vehicles(9.0, &vehicles, &map)
            .expect("second call should not error");
        assert!(spawned_again.is_empty());
    }

    #[test]
    fn scheduler_rejects_invalid_profiles() {
        let map = make_minimal_straight_map();
        let result = ShiftScheduler::new(
            &map,
            vec![ShiftProfileInput {
                origin: 0,
                destination: 2,
                departure_time: -1.0,
                dwell_time: 2.0,
            }],
        );
        assert!(result.is_err());
    }

    #[test]
    fn scheduler_rejects_duplicate_profiles() {
        let map = make_minimal_straight_map();
        let result = ShiftScheduler::new(
            &map,
            vec![
                ShiftProfileInput {
                    origin: 0,
                    destination: 2,
                    departure_time: 1.0,
                    dwell_time: 2.0,
                },
                ShiftProfileInput {
                    origin: 0,
                    destination: 2,
                    departure_time: 1.0,
                    dwell_time: 3.0,
                },
            ],
        );
        assert!(result.is_err());
    }
}
