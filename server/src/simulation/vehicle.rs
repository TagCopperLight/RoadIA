use crate::simulation::config::{ACCELERATION_EXPONENT, MAX_SPEED};
use petgraph::graph::{EdgeIndex, NodeIndex};

use crate::map::{model::Coordinates, model::Map};

#[derive(Copy, Clone)]
pub enum VehicleKind {
    Car,
    Bus,
}

#[derive(Copy, Clone)]
pub struct VehicleSpec {
    pub kind: VehicleKind,
    pub max_speed: f32,
    pub max_acceleration: f32,
    pub comfortable_deceleration: f32,
    pub reaction_time: f32,
    pub length: f32,
}

impl VehicleSpec {
    pub fn new(kind: VehicleKind, max_speed: f32, max_acceleration: f32, comfortable_deceleration: f32, reaction_time: f32, length: f32) -> Self {
        Self {
            kind,
            max_speed,
            max_acceleration,
            comfortable_deceleration,
            reaction_time,
            length,
        }
    }
}

#[derive(Clone)]
pub struct TripRequest {
    pub origin: NodeIndex,
    pub destination: NodeIndex,
    pub departure_time: f32,
}

#[derive(Copy, Clone, PartialEq, Debug, Hash, Eq)]
pub enum LaneId {
    Normal(EdgeIndex, u32), // Normal lane (EdgeIndex, lane.id).
    Internal(u32, u32), // Internal lane (intersection.id, internal_lane.id).
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum VehicleState {
    WaitingToDepart,
    OnRoad,
    Arrived,
}

#[derive(Clone)]
pub struct DrivePlanEntry {
    pub link_id: u32,
    pub lane_id: LaneId,
    pub via_internal_lane_id: u32,
    pub junction_id: u32,
    pub v_pass: f32,
    pub v_wait: f32,
    pub arrival_time: f32,
    pub arrival_speed: f32,
    pub leave_time: f32,
    pub leave_speed: f32,
    pub distance: f32,
    pub set_request: bool,
}

#[derive(Clone)]
pub struct Vehicle {
    pub id: u64,
    pub spec: VehicleSpec,
    pub trip: TripRequest,
    pub state: VehicleState,

    pub path: Vec<NodeIndex>,
    pub path_index: usize,

    pub position_on_lane: f32,
    pub velocity: f32,
    pub previous_velocity: f32,

    pub current_lane: Option<LaneId>,
    pub drive_plan: Vec<DrivePlanEntry>,
    pub registered_link_ids: Vec<u32>,
    pub waiting_time: f32,
    pub impatience: f32,

    pub emitted_co2: f32,
    pub arrived_at: Option<f32>,
}

pub fn fastest_path(map: &Map, source: NodeIndex, destination: NodeIndex) -> Vec<NodeIndex> {
    let result = petgraph::algo::astar(
        &map.graph,
        source,
        |finish| finish == destination,
        |e| e.weight().length / e.weight().speed_limit,
        |n| map.intersections_euclidean_distance(n, destination) / MAX_SPEED,
    );
    match result {
        Some((_cost, path)) => path,
        None => panic!("No path found between {:?} and {:?}", source, destination),
    }
}

impl Vehicle {
    pub fn new(id: u64, spec: VehicleSpec, trip: TripRequest) -> Self {
        Self {
            id,
            spec,
            trip,
            state: VehicleState::WaitingToDepart,
            path: Vec::new(),
            path_index: 0,
            previous_velocity: 0.0,
            velocity: 0.0,
            position_on_lane: 0.0,
            current_lane: None,
            drive_plan: Vec::new(),
            registered_link_ids: Vec::new(),
            waiting_time: 0.0,
            impatience: 0.0,
            emitted_co2: 0.0,
            arrived_at: None,
        }
    }

    pub fn update_path(&mut self, map: &Map) {
        self.path = fastest_path(map, self.trip.origin, self.trip.destination);
        self.path_index = 0;
    }

    pub fn compute_acceleration(
        &self,
        desired_velocity: f32,
        mut minimum_gap: f32,
        vehicle_ahead_distance: f32,
        vehicle_ahead_velocity: f32,
    ) -> f32 {
        if minimum_gap == 0.0 {
            minimum_gap = 0.1;
        }

        let free_road_acc = self.spec.max_acceleration
            * (1.0 - (self.previous_velocity / desired_velocity).powf(ACCELERATION_EXPONENT));

        if vehicle_ahead_distance <= 0.0 {
            return -self.spec.comfortable_deceleration;
        }

        let s_delta = 0.5 * self.previous_velocity * (self.previous_velocity - vehicle_ahead_velocity)
            / (self.spec.max_acceleration * self.spec.comfortable_deceleration).sqrt();
        let s: f32 = minimum_gap
            + self.previous_velocity * self.spec.reaction_time
            + s_delta.max(0.0);

        free_road_acc - self.spec.max_acceleration * (s / vehicle_ahead_distance).powf(2.0)
    }

    pub fn get_coordinates(&self, map: &Map) -> Coordinates {
        match self.state {
            VehicleState::OnRoad => {
                match self.current_lane {
                    Some(LaneId::Internal(junction_id, internal_lane_id)) => {
                        if let Some(&junction_node_idx) = map.node_index_map.get(&junction_id) {
                            let junction = &map.graph[junction_node_idx];
                            if let Some(il) = junction
                                .internal_lanes
                                .iter()
                                .find(|il| il.id == internal_lane_id)
                            {
                                let t = (self.position_on_lane / il.length).clamp(0.0, 1.0);
                                return Coordinates {
                                    x: il.entry.0 + (il.exit.0 - il.entry.0) * t,
                                    y: il.entry.1 + (il.exit.1 - il.entry.1) * t,
                                };
                            }
                        }
                        let node = map.graph.node_weight(self.get_current_node()).expect("node");
                        Coordinates {
                            x: node.center_coordinates.x,
                            y: node.center_coordinates.y,
                        }
                    }
                    _ => {
                        let cur = map.graph.node_weight(self.get_current_node()).expect("node");
                        let nxt = map.graph.node_weight(self.get_next_node()).expect("node");
                        let road = map
                            .graph
                            .edge_weight(
                                map.graph
                                    .find_edge(self.get_current_node(), self.get_next_node())
                                    .expect("edge"),
                            )
                            .expect("edge weight");
                            
                        let tdx = nxt.center_coordinates.x - cur.center_coordinates.x;
                        let tdy = nxt.center_coordinates.y - cur.center_coordinates.y;
                        let tlen = (tdx * tdx + tdy * tdy).sqrt();

                        let (start_x, start_y) = if tlen > 1e-6 {
                            (cur.center_coordinates.x + (tdx / tlen) * cur.radius, cur.center_coordinates.y + (tdy / tlen) * cur.radius)
                        } else {
                            (cur.center_coordinates.x, cur.center_coordinates.y)
                        };

                        let (end_x, end_y) = if tlen > 1e-6 {
                            (nxt.center_coordinates.x - (tdx / tlen) * nxt.radius, nxt.center_coordinates.y - (tdy / tlen) * nxt.radius)
                        } else {
                            (nxt.center_coordinates.x, nxt.center_coordinates.y)
                        };

                        let t = self.position_on_lane / road.length;
                        let cx = start_x * (1.0 - t) + end_x * t;
                        let cy = start_y * (1.0 - t) + end_y * t;

                        let lane_idx = match self.current_lane {
                            Some(LaneId::Normal(_, lid)) => lid as usize,
                            _ => 0,
                        };
                        
                        let (perp_x, perp_y) = if tlen > 1e-6 {
                            (-tdy / tlen, tdx / tlen)
                        } else {
                            (0.0, 0.0)
                        };
                        let offset = (lane_idx as f32 + 0.5) * road.lane_width;
                        Coordinates {
                            x: cx + perp_x * offset,
                            y: cy + perp_y * offset,
                        }
                    }
                }
            }
            _ => {
                let node = map
                    .graph
                    .node_weight(self.get_current_node())
                    .expect("node");
                Coordinates {
                    x: node.center_coordinates.x,
                    y: node.center_coordinates.y,
                }
            }
        }
    }

    pub fn get_heading(&self, map: &Map) -> f32 {
        if self.path.len() < 2 {
            return 0.0;
        }
        match self.state {
            VehicleState::OnRoad => {
                match self.current_lane {
                    Some(LaneId::Internal(junction_id, internal_lane_id)) => {
                        if let Some(&junction_node_idx) = map.node_index_map.get(&junction_id) {
                            let junction = &map.graph[junction_node_idx];
                            if let Some(il) = junction
                                .internal_lanes
                                .iter()
                                .find(|il| il.id == internal_lane_id)
                            {
                                let dx = il.exit.0 - il.entry.0;
                                let dy = il.exit.1 - il.entry.1;
                                return dy.atan2(dx);
                            }
                        }
                        0.0
                    }
                    _ => {
                        let cur = map.graph.node_weight(self.get_current_node()).expect("node");
                        let nxt = map.graph.node_weight(self.get_next_node()).expect("node");
                        let tdx = nxt.center_coordinates.x - cur.center_coordinates.x;
                        let tdy = nxt.center_coordinates.y - cur.center_coordinates.y;
                        tdy.atan2(tdx)
                    }
                }
            }
            _ => {
                // WaitingToDepart: use the direction of the first road segment
                let cur = map.graph.node_weight(self.path[0]).expect("node");
                let nxt = map.graph.node_weight(self.path[1]).expect("node");
                let tdx = nxt.center_coordinates.x - cur.center_coordinates.x;
                let tdy = nxt.center_coordinates.y - cur.center_coordinates.y;
                tdy.atan2(tdx)
            }
        }
    }

    pub fn get_current_node(&self) -> NodeIndex {
        self.path[self.path_index]
    }

    pub fn get_next_node(&self) -> NodeIndex {
        if self.path_index + 1 >= self.path.len() {
            panic!("Vehicle {} is at destination", self.id);
        }
        self.path[self.path_index + 1]
    }

    pub fn get_current_road(&self, map: &Map) -> Option<EdgeIndex> {
        match self.current_lane {
            Some(LaneId::Internal(_, _)) => None,
            _ => map
                .graph
                .find_edge(self.get_current_node(), self.get_next_node()),
        }
    }

}
