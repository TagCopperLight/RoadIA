use crate::{map::model::Map, simulation::config::MAX_SPEED_KMH};
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
    pub max_speed_kmh: f32,
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

    pub position_on_edge_m: f32,

    pub x: f32,
    pub y: f32,

    pub departure_time_s: u32,
    pub arrival_time_s: Option<u32>,

    pub distance_travelled_m: f32,
    pub fuel_used_l: f32,
    pub co2_emitted_g: f32,

    #[serde(skip)]
    pub intersection_wait_start_time_s: Option<f32>,
}

// -----------------------------------------------------------------------------
// Routing
// -----------------------------------------------------------------------------

pub fn intersections_euclidian_distance(
    map: &Map,
    source: NodeIndex,
    destination: NodeIndex,
) -> f32 {
    let n1 = &map.graph[source];
    let n2 = &map.graph[destination];
    ((n1.x - n2.x).powf(2.0) + (n1.y - n2.y).powf(2.0)).sqrt()
}
pub fn shortest_path(map: &Map, source: NodeIndex, destination: NodeIndex) -> Vec<NodeIndex> {
    let result = petgraph::algo::astar(
        &map.graph,
        source,
        |finish| finish == destination,
        |e| e.weight().length_m / (e.weight().speed_limit_kmh as f32 / 3.6),
        |n| intersections_euclidian_distance(map, n, destination) / (MAX_SPEED_KMH / 3.6),
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
    pub fn new(
        id: u64,
        spec: VehicleSpec,
        trip: TripRequest,
        initial_node: NodeIndex,
        departure_time_s: u32,
        x: f32,
        y: f32,
    ) -> Self {
        Self {
            id,
            spec,
            trip,
            state: VehicleState::WaitingToDepart,
            current_node: initial_node,
            next_node: None,
            path: Vec::new(),
            path_index: 0,
            position_on_edge_m: 0.0,
            x,
            y,
            departure_time_s,
            arrival_time_s: None,
            distance_travelled_m: 0.0,
            fuel_used_l: 0.0,
            co2_emitted_g: 0.0,
            intersection_wait_start_time_s: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::intersection::{Intersection, IntersectionKind};
    use crate::map::road::Road;

    #[test]
    fn test1() {
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

        map.add_two_way_road(
            h1,
            i3,
            Road {
                id: 1,
                lane_count: 1,
                speed_limit_kmh: 100,
                length_m: 1.,
                is_blocked: false,
                can_overtake: false,
            },
        );

        map.add_two_way_road(
            h2,
            i3,
            Road {
                id: 2,
                lane_count: 1,
                speed_limit_kmh: 100,
                length_m: 1.,
                is_blocked: false,
                can_overtake: false,
            },
        );

        map.add_two_way_road(
            i3,
            i8,
            Road {
                id: 3,
                lane_count: 1,
                speed_limit_kmh: 100,
                length_m: 5.,
                is_blocked: false,
                can_overtake: false,
            },
        );

        map.add_two_way_road(
            i3,
            i5,
            Road {
                id: 4,
                lane_count: 1,
                speed_limit_kmh: 100,
                length_m: 1.,
                is_blocked: false,
                can_overtake: false,
            },
        );

        map.add_two_way_road(
            i8,
            i6,
            Road {
                id: 5,
                lane_count: 1,
                speed_limit_kmh: 100,
                length_m: 1.,
                is_blocked: false,
                can_overtake: false,
            },
        );

        map.add_two_way_road(
            i5,
            i6,
            Road {
                id: 6,
                lane_count: 1,
                speed_limit_kmh: 100,
                length_m: 1.,
                is_blocked: false,
                can_overtake: false,
            },
        );

        map.add_two_way_road(
            i8,
            i7,
            Road {
                id: 7,
                lane_count: 1,
                speed_limit_kmh: 100,
                length_m: 2.,
                is_blocked: false,
                can_overtake: false,
            },
        );

        map.add_two_way_road(
            i7,
            i4,
            Road {
                id: 8,
                lane_count: 1,
                speed_limit_kmh: 100,
                length_m: 1.,
                is_blocked: false,
                can_overtake: false,
            },
        );

        map.add_two_way_road(
            i6,
            i4,
            Road {
                id: 9,
                lane_count: 1,
                speed_limit_kmh: 100,
                length_m: 2.,
                is_blocked: false,
                can_overtake: false,
            },
        );

        map.add_two_way_road(
            i4,
            w9,
            Road {
                id: 1,
                lane_count: 1,
                speed_limit_kmh: 100,
                length_m: 1.,
                is_blocked: false,
                can_overtake: false,
            },
        );

        let path: Vec<NodeIndex> = shortest_path(&map, h1, w9);
        assert_eq!(path, vec![h1, i3, i5, i6, i4, w9]);
    }
}
