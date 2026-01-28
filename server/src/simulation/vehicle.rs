use crate::map::intersection::Intersection;
use crate::map::model::Coordinates;
use crate::{map::model::Map, simulation::config::MAX_SPEED_MS};
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VehicleKind {
    Car,
    Bus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleSpec {
    pub kind: VehicleKind,
    pub max_speed_ms: f32,
    pub max_acceleration_ms2: f32,
    pub comfortable_deceleration: f32,
    pub reaction_time: f32,
    pub length_m: f32,
    pub fuel_consumption_l_per_100km: f32,
    pub co2_g_per_km: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TripRequest {
    pub origin_id: u64,
    pub destination_id: u64,
    pub departure_time_s: u32,
    pub return_time_s: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VehicleState {
    WaitingToDepart,
    EnRoute,
    AtIntersection,
    Arrived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vehicle {
    pub id: u64,
    pub spec: VehicleSpec,
    pub trip: TripRequest,
    pub state: VehicleState,

    #[serde(skip)]
    pub current_node: NodeIndex,

    #[serde(skip)]
    pub next_node: Option<NodeIndex>,

    #[serde(skip)]
    pub path: Vec<NodeIndex>,

    #[serde(skip)]
    pub path_index: usize,

    pub position_on_edge_m: f32, //distance entre l'avant du véhicule et la fin de la route
    pub velocity: f32,
    pub previous_velocity: f32,

    pub distance_travelled_m: f32,
    pub fuel_used_l: f32,
    pub co2_emitted_g: f32,

    #[serde(skip)]
    pub intersection_wait_start_time_s: Option<f32>,
}

// -----------------------------------------------------------------------------
// Routing
// -----------------------------------------------------------------------------

pub fn fastest_path(map: &Map, source: NodeIndex, destination: NodeIndex) -> Vec<NodeIndex> {
    let result = petgraph::algo::astar(
        &map.graph,
        source,
        |finish| finish == destination,
        |e| e.weight().length_m / f32::from(e.weight().speed_limit_ms),
        |n| map.intersections_euclidean_distance(n, destination) / f32::from(MAX_SPEED_MS),
    );
    match result {
        Some((_cost, path)) => path,
        None => Vec::new(),
    }
}

// -----------------------------------------------------------------------------
// Vehicle impl
// -----------------------------------------------------------------------------

impl Vehicle {
    pub fn new(id: u64, spec: VehicleSpec, trip: TripRequest, initial_node: NodeIndex) -> Self {
        Self {
            id,
            spec,
            trip,
            state: VehicleState::WaitingToDepart,
            current_node: initial_node,
            next_node: None,
            path: Vec::new(),
            path_index: 0,
            previous_velocity: 0.0,
            velocity: 0.0,
            position_on_edge_m: 0.0,
            distance_travelled_m: 0.0,
            fuel_used_l: 0.0,
            co2_emitted_g: 0.0,
            intersection_wait_start_time_s: None,
        }
    }

    pub fn compute_acceleration(
        &self,
        b2b_distance: f32,
        next_vehicle_velocity: f32,
        desired_velocity: f32,
        minimum_gap: f32,
        acceleration_exponent: f32,
    ) -> f32 {
        let s: f32 = minimum_gap
            + self.previous_velocity * self.spec.reaction_time
            + 0.5 * self.previous_velocity * (self.previous_velocity - next_vehicle_velocity)
                / (self.spec.max_acceleration_ms2 * self.spec.comfortable_deceleration).powf(0.5);
        let new_acceleration: f32 = self.spec.max_acceleration_ms2
            * (1.0
                - (self.previous_velocity / desired_velocity).powf(acceleration_exponent)
                - (s / b2b_distance));
        new_acceleration
    }

    pub fn get_coordinates(&self, map: &Map) -> Coordinates {
        match self.state {
            VehicleState::WaitingToDepart => {
                let current_node_o = map.graph.node_weight(self.current_node).unwrap();
                Coordinates {
                    x: current_node_o.x,
                    y: current_node_o.y,
                }
            }
            VehicleState::AtIntersection => {
                let next_node_o = map.graph.node_weight(self.next_node.unwrap()).unwrap();
                Coordinates {
                    x: next_node_o.x,
                    y: next_node_o.y,
                }
            }
            VehicleState::EnRoute => {
                let current_node_o = map.graph.node_weight(self.current_node).unwrap();
                let next_node_o = map.graph.node_weight(self.next_node.unwrap()).unwrap();
                let current_road = map
                    .graph
                    .edge_weight(
                        map.graph
                            .find_edge(self.current_node, self.next_node.unwrap())
                            .unwrap(),
                    )
                    .unwrap();

                let pos_rate: f32 = self.position_on_edge_m / current_road.length_m;
                Coordinates {
                    x: current_node_o.x * (1.0 - pos_rate) + next_node_o.x * pos_rate,
                    y: current_node_o.y * (1.0 - pos_rate) + next_node_o.y * pos_rate,
                }
            }
            VehicleState::Arrived => {
                let current_node_o: Intersection = map
                    .graph
                    .node_weight(*self.path.get(self.path.len() - 1).unwrap())
                    .unwrap()
                    .clone();
                Coordinates {
                    x: current_node_o.x,
                    y: current_node_o.y,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::intersection::{Intersection, IntersectionKind};
    use crate::map::road::Road;

    #[test]
    fn test_shortest_path() {
        let mut map: Map = Map::new();

        let h1 = map.add_intersection(Intersection {
            id: 1,
            kind: IntersectionKind::Habitation,
            name: "h1".into(),
            x: 0.,
            y: 0.,
        });

        let h2 = map.add_intersection(Intersection {
            id: 2,
            kind: IntersectionKind::Habitation,
            name: "h2".into(),
            x: 0.,
            y: 100.,
        });

        let i3 = map.add_intersection(Intersection {
            id: 3,
            kind: IntersectionKind::Intersection,
            name: "i3".into(),
            x: 50.,
            y: 50.,
        });

        let i4 = map.add_intersection(Intersection {
            id: 4,
            kind: IntersectionKind::Intersection,
            name: "i4".into(),
            x: 250.,
            y: 50.,
        });

        let i5 = map.add_intersection(Intersection {
            id: 5,
            kind: IntersectionKind::Intersection,
            name: "i5".into(),
            x: 100.,
            y: 100.,
        });

        let i6 = map.add_intersection(Intersection {
            id: 6,
            kind: IntersectionKind::Intersection,
            name: "i6".into(),
            x: 150.,
            y: 50.,
        });

        let i7 = map.add_intersection(Intersection {
            id: 7,
            kind: IntersectionKind::Intersection,
            name: "i7".into(),
            x: 200.,
            y: 50.,
        });

        let i8 = map.add_intersection(Intersection {
            id: 8,
            kind: IntersectionKind::Intersection,
            name: "i8".into(),
            x: 100.,
            y: 0.,
        });

        let w9 = map.add_intersection(Intersection {
            id: 9,
            kind: IntersectionKind::Workplace,
            name: "w9".into(),
            x: 300.,
            y: 50.,
        });

        map.add_two_way_road(h1, i3, Road::new(1, 1, 100, 1., false, false));

        map.add_two_way_road(h2, i3, Road::new(2, 1, 100, 1., false, false));

        map.add_two_way_road(i3, i8, Road::new(3, 1, 100, 5., false, false));

        map.add_two_way_road(i3, i5, Road::new(4, 1, 100, 1., false, false));

        map.add_two_way_road(i8, i6, Road::new(5, 1, 100, 1., false, false));

        map.add_two_way_road(i5, i6, Road::new(6, 1, 100, 1., false, false));

        map.add_two_way_road(i8, i7, Road::new(7, 1, 100, 2., false, false));

        map.add_two_way_road(i7, i4, Road::new(8, 1, 100, 1., false, false));

        map.add_two_way_road(i6, i4, Road::new(9, 1, 100, 2., false, false));

        map.add_two_way_road(i4, w9, Road::new(10, 1, 100, 1., false, false));

        let path: Vec<NodeIndex> = fastest_path(&map, h1, w9);
        assert_eq!(path, vec![h1, i3, i5, i6, i4, w9]);
    }
}
