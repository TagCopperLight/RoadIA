//! Trip lifecycle: departures, arrivals, commutes, bus loops and route changes.

use std::cmp::Reverse;

use ordered_float::OrderedFloat;
use petgraph::graph::NodeIndex;
use rand::Rng;

use crate::map::model::{Map, SavedBusLine};
use crate::simulation::commute::CommutePlanState;
use crate::simulation::config::NAVIGATION_MISTAKE_PROBABILITY;
use crate::simulation::vehicle::{
    route_through, LaneId, TripRequest, Vehicle, VehicleKind, VehicleSpec, VehicleState,
    VehicleType,
};

use super::network::LinkInfo;
use super::{BusLine, SimulationEngine};

impl SimulationEngine {
    pub(super) fn enqueue_departure(&mut self, vidx: usize) {
        let vehicle = &self.vehicles[vidx];
        if vehicle.state != VehicleState::WaitingToDepart || vehicle.trip.departure_time >= f32::MAX {
            return;
        }
        self.departure_queue
            .push(Reverse((OrderedFloat(vehicle.trip.departure_time), vidx)));
    }

    pub(super) fn handle_departures(&mut self) {
        let mut blocked = Vec::new();
        while let Some(&Reverse((departure_time, vidx))) = self.departure_queue.peek() {
            if departure_time > OrderedFloat(self.current_time) {
                break;
            }
            self.departure_queue.pop();

            let vehicle = &self.vehicles[vidx];
            // Skip stale entries (the trip was rescheduled since it was queued).
            if vehicle.state != VehicleState::WaitingToDepart
                || OrderedFloat(vehicle.trip.departure_time) != departure_time
            {
                continue;
            }
            if !self.start_trip(vidx) {
                blocked.push(vidx);
            }
        }

        for vidx in blocked {
            self.enqueue_departure(vidx);
        }
    }

    /// Puts the vehicle at the start of its first road, on the lane with the most room.
    fn start_trip(&mut self, vidx: usize) -> bool {
        let path = &self.vehicles[vidx].path;
        if path.len() < 2 {
            return false;
        }
        let Some(edge) = self.config.map.graph.find_edge(path[0], path[1]) else {
            return false;
        };

        // Space left at the start of the lane, minus what vehicles crossing the junction
        // into it already claimed.
        let room = |lane: LaneId| {
            let claimed = self.exit_claims.get(&lane).copied().unwrap_or(0.0);
            match self.rear_of_lane(lane) {
                Some(rear) => {
                    let rv = &self.vehicles[rear];
                    rv.position_on_lane - rv.spec.length - claimed
                }
                None if claimed == 0.0 => f32::INFINITY,
                None => self.lane_length(lane) - claimed,
            }
        };
        let best = self.config.map.graph[edge]
            .lanes
            .iter()
            .map(|lane| LaneId::Normal(edge, lane.id))
            .map(|lane| (lane, room(lane)))
            .max_by(|a, b| a.1.total_cmp(&b.1));
        let Some((lane, room)) = best else {
            return false;
        };
        if room < self.config.minimum_gap {
            return false;
        }

        let vehicle = &mut self.vehicles[vidx];
        vehicle.position_on_lane = 0.0;
        vehicle.state = VehicleState::OnRoad;
        self.move_vehicle_to_lane(vidx, lane);

        let vehicle = &self.vehicles[vidx];
        if let Some(plan) = vehicle.commute_plan_id.and_then(|id| self.commute_plans.get_mut(&id)) {
            if plan.outbound_vehicle_id == vehicle.id {
                plan.state = CommutePlanState::OutboundRunning;
            } else if plan.return_vehicle_id == Some(vehicle.id) {
                plan.state = CommutePlanState::ReturnRunning;
            }
        }
        true
    }

    /// The vehicle reached the end of its last road.
    pub(super) fn finish_trip(&mut self, vidx: usize) {
        self.unregister(vidx);
        if let Some(lane) = self.vehicles[vidx].current_lane {
            self.remove_vehicle_from_lane(lane, vidx);
        }
        self.queue_ranks[vidx] = None;

        if matches!(self.vehicles[vidx].spec.kind, VehicleKind::Bus) {
            self.restart_bus_loop(vidx);
            return;
        }

        let road_length = match self.vehicles[vidx].current_lane {
            Some(lane) => self.lane_length(lane),
            None => 0.0,
        };
        let vehicle = &mut self.vehicles[vidx];
        vehicle.position_on_lane = road_length;
        vehicle.drive_plan.clear();
        vehicle.state = VehicleState::Arrived;
        vehicle.arrived_at = Some(self.current_time);
        self.handle_vehicle_arrival(vidx);
    }

    fn restart_bus_loop(&mut self, vidx: usize) {
        let stops = self
            .bus_lines
            .iter()
            .find(|line| line.vehicle_id == self.vehicles[vidx].id)
            .map(|line| stop_nodes(&self.config.map, &line.stop_node_ids));

        let vehicle = &mut self.vehicles[vidx];
        vehicle.state = VehicleState::WaitingToDepart;
        vehicle.path_index = 0;
        vehicle.position_on_lane = 0.0;
        vehicle.velocity = 0.0;
        vehicle.current_lane = None;
        vehicle.drive_plan.clear();
        vehicle.waiting_time = 0.0;
        vehicle.impatience = 0.0;
        vehicle.trip.departure_time = self.current_time;
        if let Some(stops) = stops {
            vehicle.waypoints = stops.get(1..).unwrap_or_default().to_vec();
        }
        self.enqueue_departure(vidx);
    }

    pub(super) fn refresh_all_arrived(&mut self) {
        let mut cars = self
            .vehicles
            .iter()
            .filter(|vehicle| !matches!(vehicle.spec.kind, VehicleKind::Bus))
            .peekable();
        self.all_vehicles_arrived =
            cars.peek().is_some() && cars.all(|vehicle| vehicle.state == VehicleState::Arrived);
    }

    fn handle_vehicle_arrival(&mut self, vidx: usize) {
        self.schedule_commute_return(vidx);
        self.refresh_all_arrived();
    }

    fn schedule_commute_return(&mut self, vidx: usize) {
        let vehicle_id = self.vehicles[vidx].id;
        let Some(plan_id) = self.vehicles[vidx].commute_plan_id else {
            return;
        };
        let Some(plan) = self.commute_plans.get(&plan_id).cloned() else {
            return;
        };

        match plan.state {
            CommutePlanState::ReturnRunning if plan.return_vehicle_id == Some(vehicle_id) => {
                self.commute_plans.get_mut(&plan_id).unwrap().state = CommutePlanState::Completed;
            }
            CommutePlanState::OutboundPending | CommutePlanState::OutboundRunning
                if vehicle_id == plan.outbound_vehicle_id =>
            {
                let ready_at = self.vehicles[vidx].arrived_at.unwrap_or(self.current_time)
                    + plan.return_waiting_time_s;
                // The return vehicle must be this plan's own, still parked vehicle.
                let return_idx = plan
                    .return_vehicle_id
                    .and_then(|id| self.vehicle_indices_by_id.get(&id).copied())
                    .filter(|&idx| {
                        let candidate = &self.vehicles[idx];
                        idx != vidx
                            && candidate.commute_plan_id == Some(plan_id)
                            && candidate.state == VehicleState::WaitingToDepart
                    });

                let scheduled = return_idx.is_some_and(|return_idx| {
                    let return_vehicle = &mut self.vehicles[return_idx];
                    return_vehicle.trip.departure_time = ready_at;
                    return_vehicle.state = VehicleState::WaitingToDepart;
                    return_vehicle.position_on_lane = 0.0;
                    return_vehicle.velocity = 0.0;
                    return_vehicle.previous_velocity = 0.0;
                    return_vehicle.current_lane = None;
                    return_vehicle.drive_plan.clear();
                    return_vehicle.waiting_time = 0.0;
                    return_vehicle.impatience = 0.0;
                    if return_vehicle.update_path(&self.config.map) {
                        true
                    } else {
                        return_vehicle.trip.departure_time = f32::MAX;
                        false
                    }
                });

                self.commute_plans.get_mut(&plan_id).unwrap().state = if scheduled {
                    CommutePlanState::WaitingForReturnDeparture
                } else {
                    CommutePlanState::Failed
                };
                if let (true, Some(return_idx)) = (scheduled, return_idx) {
                    self.enqueue_departure(return_idx);
                }
            }
            _ => {}
        }
    }

    /// With a small probability, takes another exit than planned at the junction ahead
    /// and reroutes from there. Returns the link actually taken.
    pub(super) fn maybe_navigation_mistake(&mut self, vidx: usize, planned: u32) -> Option<u32> {
        if self.rng.random::<f32>() >= NAVIGATION_MISTAKE_PROBABILITY {
            return None;
        }

        let planned = self.network.link(planned)?;
        let LaneId::Normal(in_edge, in_lane) = planned.in_lane else {
            return None;
        };
        let lane = self.config.map.graph[in_edge]
            .lanes
            .iter()
            .find(|lane| lane.id == in_lane)?;
        let alternatives: Vec<u32> = lane
            .links
            .iter()
            .filter(|link| link.destination_road_id != planned.link.destination_road_id)
            .filter_map(|link| self.network.link(link.id))
            .filter(|info| !self.crossing_traffic_in_junction(info))
            .map(|info| info.link.id)
            .collect();
        if alternatives.is_empty() {
            return None;
        }

        let wrong = alternatives[self.rng.random_range(0..alternatives.len())];
        let next = self.exit_node(wrong)?;
        let junction_pos = self.vehicles[vidx].path_index + 1;
        let route = self.route_after(vidx, junction_pos, next)?;
        self.splice_route(vidx, junction_pos, route);
        Some(wrong)
    }

    /// The route asks for a turn that does not exist at the next junction (e.g. a U-turn
    /// at a bus stop): pick the exit with the fastest way to the remaining stops.
    pub(super) fn reroute_at_missing_turn(&mut self, vidx: usize) -> bool {
        let map = &self.config.map;
        let vehicle = &self.vehicles[vidx];
        let first_road = match vehicle.current_lane {
            Some(LaneId::Normal(..)) => vehicle.path_index,
            Some(LaneId::Internal(..)) => vehicle.path_index + 1,
            None => return false,
        };
        let junction_pos = first_road + 1;
        let (Some(&from), Some(&junction)) =
            (vehicle.path.get(first_road), vehicle.path.get(junction_pos))
        else {
            return false;
        };
        let Some(in_edge) = map.graph.find_edge(from, junction) else {
            return false;
        };

        let mut exits: Vec<NodeIndex> = map.graph[in_edge]
            .lanes
            .iter()
            .flat_map(|lane| &lane.links)
            .filter_map(|link| self.exit_node(link.id))
            .collect();
        exits.sort_unstable();
        exits.dedup();

        let best = exits
            .into_iter()
            .filter_map(|next| {
                let route = self.route_after(vidx, junction_pos, next)?;
                let first_leg = map.graph.find_edge(junction, next)?;
                Some((travel_cost(map, &route) + edge_cost(map, first_leg), route))
            })
            .min_by(|a, b| a.0.total_cmp(&b.0));

        match best {
            Some((_, route)) => {
                self.splice_route(vidx, junction_pos, route);
                true
            }
            None => false,
        }
    }

    /// Route from the junction at `junction_pos` of the vehicle's path through `next` to the
    /// remaining waypoints and the destination (starting at `next`). Only routes whose every
    /// turn exists are returned.
    fn route_after(&self, vidx: usize, junction_pos: usize, next: NodeIndex) -> Option<Vec<NodeIndex>> {
        let map = &self.config.map;
        let vehicle = &self.vehicles[vidx];
        let junction = *vehicle.path.get(junction_pos)?;
        let waypoints: Vec<NodeIndex> =
            vehicle.waypoints.iter().copied().filter(|&w| w != junction).collect();
        let route = route_through(map, next, &waypoints, vehicle.trip.destination)?;

        let drivable = std::iter::once(junction)
            .chain(route.iter().copied())
            .collect::<Vec<_>>()
            .windows(3)
            .all(|turn| {
                let in_edge = map.graph.find_edge(turn[0], turn[1]);
                let out_edge = map.graph.find_edge(turn[1], turn[2]);
                matches!((in_edge, out_edge), (Some(in_edge), Some(out_edge))
                    if self.network.choose_link(map, in_edge, 0, out_edge).is_some())
            });
        drivable.then_some(route)
    }

    /// Keeps the path up to the junction at `junction_pos` and continues with `route`.
    fn splice_route(&mut self, vidx: usize, junction_pos: usize, route: Vec<NodeIndex>) {
        let vehicle = &mut self.vehicles[vidx];
        vehicle.path.truncate(junction_pos + 1);
        vehicle.path.extend(route);
    }

    fn exit_node(&self, link_id: u32) -> Option<NodeIndex> {
        match self.network.link(link_id)?.out_lane {
            LaneId::Normal(edge, _) => self.config.map.graph.edge_endpoints(edge).map(|(_, to)| to),
            LaneId::Internal(..) => None,
        }
    }

    fn crossing_traffic_in_junction(&self, info: &LinkInfo) -> bool {
        info.link
            .foe_links
            .iter()
            .filter_map(|foe| self.network.link(foe.id))
            .any(|foe| self.vehicles_by_lane.contains_key(&foe.internal_lane))
    }

    pub fn add_bus_from_saved(&mut self, line: &SavedBusLine) -> bool {
        let stops = stop_nodes(&self.config.map, &line.stop_node_ids);
        if stops.len() < 2 {
            return false;
        }
        let spec = VehicleSpec::new(VehicleKind::Bus, 8.0, 2.0, 3.0, 1.0, 12.0);
        let trip = TripRequest { origin: stops[0], destination: stops[0], departure_time: 0.0 };
        let vehicle_id = self.next_vehicle_id;
        let mut bus = Vehicle::new(vehicle_id, spec, trip, VehicleType::Electrique);
        bus.waypoints = stops[1..].to_vec();
        if !bus.update_path(&self.config.map) {
            return false;
        }

        self.next_vehicle_id += 1;
        let vidx = self.vehicles.len();
        self.vehicles.push(bus);
        self.lane_slots.push(None);
        self.queue_ranks.push(None);
        self.vehicle_indices_by_id.insert(vehicle_id, vidx);
        self.bus_lines.push(BusLine {
            id: line.id,
            name: line.name.clone(),
            stop_node_ids: line.stop_node_ids.clone(),
            vehicle_id,
        });
        self.enqueue_departure(vidx);
        true
    }
}

fn stop_nodes(map: &Map, stop_ids: &[u32]) -> Vec<NodeIndex> {
    stop_ids
        .iter()
        .filter_map(|id| map.node_index_map.get(id).copied())
        .collect()
}

fn edge_cost(map: &Map, edge: petgraph::graph::EdgeIndex) -> f32 {
    let road = &map.graph[edge];
    road.length / road.speed_limit
}

fn travel_cost(map: &Map, route: &[NodeIndex]) -> f32 {
    route
        .windows(2)
        .filter_map(|pair| map.graph.find_edge(pair[0], pair[1]))
        .map(|edge| edge_cost(map, edge))
        .sum()
}
