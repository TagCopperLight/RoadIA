use petgraph::graph::{EdgeIndex, Graph, NodeIndex};

use crate::map::intersection::Intersection;
use crate::map::road::Road;

use crate::simulation::config::{SimulationConfig};
use crate::simulation::vehicle::{VehicleSpec};

#[derive(Default, Clone)]
pub struct Map {
    pub graph: Graph<Intersection, Road>,
}

pub struct Coordinates{
    pub x : f32,
    pub y : f32,
}

impl Map {
    pub fn new() -> Self {
        Self {
            graph: Graph::new(),
        }
    }

    pub fn add_intersection(&mut self, intersection: Intersection) -> NodeIndex {
        self.graph.add_node(intersection)
    }

    pub fn add_road(&mut self, from: NodeIndex, to: NodeIndex, road: Road) -> EdgeIndex {
        let rule = match self.graph[to].intersection_type {
            crate::map::intersection::IntersectionType::Priority => crate::map::intersection::IntersectionRules::Priority,
            crate::map::intersection::IntersectionType::Stop => crate::map::intersection::IntersectionRules::Stop,
            crate::map::intersection::IntersectionType::TrafficLight => crate::map::intersection::IntersectionRules::TrafficLight,
        };
        self.graph[to].set_rule(road.id, rule);
        self.graph.add_edge(from, to, road)
    }

    pub fn add_two_way_road(
        &mut self,
        from: NodeIndex,
        to: NodeIndex,
        road: Road,
    ) -> (EdgeIndex, EdgeIndex) {
        let e1 = self.add_road(from, to, road.clone());
        let e2 = self.add_road(to, from, road);
        (e1, e2)
    }

    pub fn neighboring_intersections(&self, source: NodeIndex) -> Vec<NodeIndex> {
        self.graph.neighbors(source).collect()
    }

    pub fn intersection_neighbor_distance(
        &self,
        source: NodeIndex,
        destination: NodeIndex,
    ) -> Option<f32> {
        self.graph
            .find_edge(source, destination)
            .map(|edge| self.graph[edge].length)
    }

    pub fn intersections_euclidean_distance(
        &self,
        source: NodeIndex,
        destination: NodeIndex,
    ) -> f32 {
        let n1 = &self.graph[source];
        let n2 = &self.graph[destination];
        let dx = n1.x - n2.x;
        let dy = n1.y - n2.y;
        (dx * dx + dy * dy).sqrt()
    }

    pub fn get_minimal_time_travel_by_road(&self, road_index : EdgeIndex, acceleration : f32, vehicle_max_speed : f32) -> f32{
        match self.graph.edge_weight(road_index) {
            Some(road) => {
                let max_speed = vehicle_max_speed.min(road.speed_limit);
                let acceleration_phase_length = 0.5 * max_speed * max_speed / acceleration;
                if road.length <= acceleration_phase_length {
                    (2.0 * road.length / acceleration).sqrt()
                }else{
                    max_speed / acceleration + (road.length - acceleration_phase_length) / max_speed
                }
            },
            None => 0.0,
        }
    }

    pub fn get_minimal_co2_by_road(&self, road_index : EdgeIndex, vehicle_spec : VehicleSpec, simulation_config : &SimulationConfig) -> f32 {
        match self.graph.edge_weight(road_index){
            Some(road) => {
                let max_speed = vehicle_spec.max_speed.min(road.speed_limit);
                let acceleration_phase_length = 0.5 * max_speed * max_speed / vehicle_spec.max_acceleration;
                //Les 3 coefficients suivants sont des constantes posées dans la doc
                let c1 = vehicle_spec.stoichiometric_co2_factor / (vehicle_spec.engine_thermal_efficiency * vehicle_spec.lower_heating_value_for_fuel);
                let c2 = 0.5 * simulation_config.air_density * vehicle_spec.aerodynamic_drag_coefficient * vehicle_spec.front_area;
                let c3 = vehicle_spec.mass * simulation_config.gravity_coefficient * vehicle_spec.rolling_resistance_coefficient;
                let t1p1 = (2.0 * road.length / vehicle_spec.max_acceleration).powf(0.5);
                let t1 = max_speed / vehicle_spec.max_acceleration;
                let t2 = (road.length - acceleration_phase_length) / max_speed;
                //println!("max_speed : {}; a: {}, l : {}, l1 : {}; c1: {}, c2: {}, c3: {}, t1: {}, t2: {}", max_speed, vehicle_spec.max_acceleration, road.length, acceleration_phase_length, c1, c2, c3, t1, t2);
                if acceleration_phase_length >= road.length{
                    0.5 * c1 * (c2 * vehicle_spec.max_acceleration.powi(3) * 0.5 * t1p1.powi(4) + c3 * vehicle_spec.max_acceleration * t1p1.powi(2) + vehicle_spec.mass * vehicle_spec.max_acceleration.powi(2) * t1p1.powi(2))
                }else{
                    t2 * c1 * (c2 * max_speed.powi(3) + c3 * max_speed) + 0.5 * c1 * (c2 * vehicle_spec.max_acceleration.powi(3) * 0.5 * t1.powi(4) + c3 * vehicle_spec.max_acceleration * t1.powi(2) + vehicle_spec.mass * vehicle_spec.max_acceleration.powi(2) * t1.powi(2))
                }
            },
            None => 0.0,
        }
    }
}
