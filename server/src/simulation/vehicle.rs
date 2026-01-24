use crate::map::model::{Map, Node};
use crate::map::road::{Road, RoadType};
use crate::map::intersection::{Intersection, IntersectionKind};
use petgraph::graph::NodeIndex;
use serde::{Deserialize, Serialize};
use std::collections::BinaryHeap;
use std::cmp::Reverse;
use std::collections::{HashSet, HashMap};
use std::cmp::Ordering;


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
    // Renamed from AgentState
    WaitingToDepart,
    EnRoute,
    AtIntersection,
    Arrived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vehicle {
    // Renamed from Agent
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

#[derive(Debug, Clone)]
struct State {
    cost: f32,
    node: NodeIndex,
}



impl PartialEq for State {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
    }
}

// Eq est un marqueur → impl vide
impl Eq for State {}

impl PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for State {
    fn cmp(&self, other: &Self) -> Ordering {
        // inversion pour min-heap
        other.cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
    }
}

pub fn road_euclidian_distance(map: &Map, road_index: NodeIndex) -> f32{
    let neighbors : Vec<NodeIndex> = map.graph.neighbors(road_index).collect();
    if let Node::Intersection(n1) = &map.graph[neighbors[0]]{
        if let Node::Intersection(n2) = &map.graph[neighbors[1]]{
            return ((n1.x - n2.x).powf(2.0) + (n1.y - n2.y).powf(2.0)).sqrt();
        };
    };
    return -67.0;//pour le compilo
}

pub fn intersections_euclidian_distance(map: &Map, source: NodeIndex, destination: NodeIndex) -> f32{
    if let Node::Intersection(n1) = &map.graph[source]{
        if let Node::Intersection(n2) = &map.graph[destination]{
            return ((n1.x - n2.x).powf(2.0) + (n1.y - n2.y).powf(2.0)).sqrt();
        };
    };
    return -67.0;//pour le compilo
}

pub fn rebuild_path(pred : &HashMap<NodeIndex, NodeIndex>, source : NodeIndex, destination : NodeIndex) -> Vec<NodeIndex>{
    let mut path : Vec<NodeIndex> = Vec::new();
    path.push(destination);
    let mut current = destination;
    while pred.contains_key(&current){
        current = *pred.get(&current).unwrap();
        path.insert(0, current);
    }
    return path;
}

pub fn shortest_path(map: &Map, source: NodeIndex, destination: NodeIndex) -> Vec<NodeIndex>{
    let mut file_prio_min = BinaryHeap::new();
    let mut prios : HashMap<NodeIndex, f32> = HashMap::new();
    let mut parcourus : HashSet<NodeIndex> = HashSet::new();
    let mut pred : HashMap<NodeIndex, NodeIndex> = HashMap::new();
    let mut distances : HashMap<NodeIndex, f32> = HashMap::new();

    file_prio_min.push(State{cost:intersections_euclidian_distance(&map, source, destination), node:source});
    prios.insert(source, intersections_euclidian_distance(&map, source, destination));
    distances.insert(source, 0.0);
    while (! file_prio_min.is_empty()){
        let n = file_prio_min.peek().unwrap().node;
        if n == destination{
            return rebuild_path(&pred, source, destination);
        }
        for node in map.neighboring_intersections(n){
            let distance_node = *distances.get(&n).expect("distance manquante") + map.intersection_neighbor_distance(n, node);
            if !parcourus.contains(&node) || distance_node < *distances.get(&node).expect("Noeud absent"){
                distances.insert(node, distance_node);
                pred.insert(node, n);

                if ! prios.contains_key(&node){
                    file_prio_min.push(State{cost: intersections_euclidian_distance(&map, node, destination), node:node});
                    prios.insert(node, intersections_euclidian_distance(&map, node, destination));
                }
            }
        }
        parcourus.insert(n);
    }

    return Vec::new();
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
mod tests{
    use super::*;

    #[test]
    fn test1(){
        let mut map : Map = Map::new();
        
        let h1 = map.add_intersection(
            Intersection{
                id: 1,
                kind: IntersectionKind::Habitation,
                name: "h1".into(),
                x: 0.,
                y: 0.,
            }
        );

        let h2 = map.add_intersection(
            Intersection{
                id: 2,
                kind: IntersectionKind::Habitation,
                name: "h2".into(),
                x: 0.,
                y: 100.,
            }
        );

        let i3 = map.add_intersection(
            Intersection{
                id: 3,
                kind: IntersectionKind::Intersection,
                name: "i3".into(),
                x: 50.,
                y: 50.,
            }
        );

        let i4 = map.add_intersection(
            Intersection{
                id: 4,
                kind: IntersectionKind::Intersection,
                name: "i4".into(),
                x: 250.,
                y: 50.,
            }
        );

        let i5 = map.add_intersection(
            Intersection{
                id: 5,
                kind: IntersectionKind::Intersection,
                name: "i5".into(),
                x: 100.,
                y: 100.,
            }
        );

        let i6 = map.add_intersection(
            Intersection{
                id: 6,
                kind: IntersectionKind::Intersection,
                name: "i6".into(),
                x: 150.,
                y: 50.,
            }
        );

        let i7 = map.add_intersection(
            Intersection{
                id: 7,
                kind: IntersectionKind::Intersection,
                name: "i7".into(),
                x: 200.,
                y: 50.,
            }
        );

        let i8 = map.add_intersection(
            Intersection{
                id: 8,
                kind: IntersectionKind::Intersection,
                name: "i8".into(),
                x: 100.,
                y: 0.,
            }
        );

        let w9 = map.add_intersection(
            Intersection{
                id: 9,
                kind: IntersectionKind::Intersection,
                name: "w9".into(),
                x: 300.,
                y: 50.,
            }
        );

        map.add_road(
            h1,
            i3,
            Road{
                id:1,
                road_type: RoadType::Bilateral,
                lanes:1,
                max_speed_kmh:100.0,
                length_m: 1.,
                is_blocked: false,
            }
        );

        map.add_road(
            h2,
            i3,
            Road{
                id:2,
                road_type: RoadType::Bilateral,
                lanes:1,
                max_speed_kmh:100.0,
                length_m: 1.,
                is_blocked: false,
            }
        );

        map.add_road(
            i3,
            i8,
            Road{
                id:3,
                road_type: RoadType::Bilateral,
                lanes:1,
                max_speed_kmh:100.0,
                length_m: 5.,
                is_blocked: false,
            }
        );

        map.add_road(
            i3,
            i5,
            Road{
                id:4,
                road_type: RoadType::Bilateral,
                lanes:1,
                max_speed_kmh:100.0,
                length_m: 1.,
                is_blocked: false,
            }
        );

        map.add_road(
            i8,
            i6,
            Road{
                id:5,
                road_type: RoadType::Bilateral,
                lanes:1,
                max_speed_kmh:100.0,
                length_m: 1.,
                is_blocked: false,
            }
        );

        map.add_road(
            i5,
            i6,
            Road{
                id:6,
                road_type: RoadType::Bilateral,
                lanes:1,
                max_speed_kmh:100.0,
                length_m: 1.,
                is_blocked: false,
            }
        );

        map.add_road(
            i8,
            i7,
            Road{
                id:7,
                road_type: RoadType::Bilateral,
                lanes:1,
                max_speed_kmh:100.0,
                length_m: 2.,
                is_blocked: false,
            }
        );

        map.add_road(
            i7,
            i4,
            Road{
                id:8,
                road_type: RoadType::Bilateral,
                lanes:1,
                max_speed_kmh:100.0,
                length_m: 1.,
                is_blocked: false,
            }
        );

        map.add_road(
            i6,
            i4,
            Road{
                id:9,
                road_type: RoadType::Bilateral,
                lanes:1,
                max_speed_kmh:100.0,
                length_m: 2.,
                is_blocked: false,
            }
        );

        map.add_road(
            i4,
            w9,
            Road{
                id:1,
                road_type: RoadType::Bilateral,
                lanes:1,
                max_speed_kmh:100.0,
                length_m: 1.,
                is_blocked: false,
            }
        );

        let mut path : Vec<NodeIndex> = shortest_path(&map, h1, w9);
        assert_eq!(path, vec![h1, i3, i5, i6, i4, w9]);
    }
}