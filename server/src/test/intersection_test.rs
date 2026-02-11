use crate::map::model::Map;
use crate::map::intersection::{Intersection, IntersectionKind, Roundabout, RoadRule, RoundaboutKind};
use crate::map::road::Road;
#[allow(unused_imports)]
use petgraph::graph::NodeIndex;

/// Creates a map with a single roundabout and 4 branches (North, East, South, West)
pub fn create_roundabout_map() -> Map {
    let mut map = Map::new();

    // 1. Create the Roundabout configuration
    let roundabout = Roundabout::new(
        1, 
        "RondPoint-Central".to_string(), 
        0.0, 
        0.0, 
        40.0, // Rayon de 40m pour correspondre à environ R_RING_OUTER visualisé
        RoundaboutKind::Standard
    );

    // 2. Create external intersections (inputs/outputs)
    let north = map.add_intersection(Intersection::new(101, IntersectionKind::Habitation, "North".to_string(), 0.0, 100.0));
    let east = map.add_intersection(Intersection::new(102, IntersectionKind::Habitation, "East".to_string(), 100.0, 0.0));
    let south = map.add_intersection(Intersection::new(103, IntersectionKind::Habitation, "South".to_string(), 0.0, -100.0));
    let west = map.add_intersection(Intersection::new(104, IntersectionKind::Habitation, "West".to_string(), -100.0, 0.0));

    // 3. Define the connections (clockwise or counter-clockwise doesn't matter for the input list, 
    // the builder sorts them by angle)
    let connections = vec![north, east, south, west];

    // 4. Build the roundabout in the map
    let _ring_nodes = roundabout.build(&mut map, connections);

    map
}

pub fn create_gyratory_roundabout_map() -> Map {
    let mut map = Map::new();

    // 1. Create the Roundabout configuration
    let roundabout = Roundabout::new(
        1, 
        "RondPoint-Gyratoire".to_string(), 
        0.0, 
        0.0, 
        40.0, 
        RoundaboutKind::Gyratory // Spécifique : Carrefour à sens giratoire (Prio Droite)
    );

    // 2. Create external intersections
    let north = map.add_intersection(Intersection::new(101, IntersectionKind::Habitation, "North".to_string(), 0.0, 100.0));
    let east = map.add_intersection(Intersection::new(102, IntersectionKind::Habitation, "East".to_string(), 100.0, 0.0));
    let south = map.add_intersection(Intersection::new(103, IntersectionKind::Habitation, "South".to_string(), 0.0, -100.0));
    let west = map.add_intersection(Intersection::new(104, IntersectionKind::Habitation, "West".to_string(), -100.0, 0.0));

    let connections = vec![north, east, south, west];
    let _ring_nodes = roundabout.build(&mut map, connections);

    map
}

pub fn create_standard_intersection_map() -> Map {
    let mut map = Map::new();

    let inter = map.add_intersection(Intersection::new(1, IntersectionKind::Intersection, "Intersection".to_string(), 0.0, 0.0));
    let h_north = map.add_intersection(Intersection::new(2, IntersectionKind::Habitation, "H-North".to_string(), 0.0, 100.0));
    let h_east = map.add_intersection(Intersection::new(3, IntersectionKind::Habitation, "H-East".to_string(), 100.0, 0.0));
    let h_south = map.add_intersection(Intersection::new(4, IntersectionKind::Habitation, "H-South".to_string(), 0.0, -100.0));
    let h_west = map.add_intersection(Intersection::new(5, IntersectionKind::Habitation, "H-West".to_string(), -100.0, 0.0));

    map.add_two_way_road(h_north, inter, Road::new(1, 1, 12, 100.0, false, false));
    map.add_two_way_road(h_east, inter, Road::new(2, 1, 12, 100.0, false, false));
    map.add_two_way_road(h_south, inter, Road::new(3, 1, 12, 100.0, false, false));
    map.add_two_way_road(h_west, inter, Road::new(4, 1, 12, 100.0, false, false));

    map
}

use serde_json::json;

pub fn get_test_scenarios() -> Vec<serde_json::Value> {
    vec![
        json!({
            "id": 1,
            "name": "FIFO (Tout Droit)",
            "vehicles": [
                {"id": 0, "name": "V0 (Sud->Nord)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Nord->Sud)", "entry_angle": 0.0, "exit_angle": 180.0, "arrival_time": 5.0}
            ],
            "authorized": [0]
        }),
        json!({
            "id": 2,
            "name": "Conflit Direct (Départage ID)",
            "vehicles": [
                {"id": 2, "name": "V2 (Sud->Nord)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 10.0},
                {"id": 5, "name": "V5 (Nord->Sud)", "entry_angle": 0.0, "exit_angle": 180.0, "arrival_time": 10.0}
            ],
            "authorized": [2]
        }),
        json!({
            "id": 3,
            "name": "Virages à Droite Simultanés",
            "vehicles": [
                {"id": 0, "name": "V0 (Sud->Est)", "entry_angle": 180.0, "exit_angle": 90.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Nord->Ouest)", "entry_angle": 0.0, "exit_angle": 270.0, "arrival_time": 0.0}
            ],
            "authorized": [0, 1]
        }),
        json!({
            "id": 4,
            "name": "Priorité à Droite (3 voies)",
            "vehicles": [
                {"id": 0, "name": "V0 (Ouest->Sud)", "entry_angle": 270.0, "exit_angle": 180.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Nord->Est)", "entry_angle": 0.0, "exit_angle": 90.0, "arrival_time": 0.0},
                {"id": 2, "name": "V2 (Est->Nord)", "entry_angle": 90.0, "exit_angle": 0.0, "arrival_time": 0.0}
            ],
            "authorized": [0, 2]
        }),
        json!({
            "id": 5,
            "name": "Face-à-Face (Tout Droit)",
            "vehicles": [
                {"id": 0, "name": "V0 (Sud->Nord)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Nord->Sud)", "entry_angle": 0.0, "exit_angle": 180.0, "arrival_time": 0.0}
            ],
            "authorized": [0, 1]
        }),
        json!({
            "id": 6,
            "name": "Virage Gauche (Prioritaire) vs Tout Droit",
            "vehicles": [
                {"id": 0, "name": "V0 (Sud->Ouest : Gauche)", "entry_angle": 180.0, "exit_angle": 270.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Nord->Sud : Tout Droit)", "entry_angle": 0.0, "exit_angle": 180.0, "arrival_time": 0.0}
            ],
            "authorized": [1]
        }),
        json!({
            "id": 7,
            "name": "Virage Droite Prioritaire",
            "vehicles": [
                {"id": 0, "name": "V0 (Sud->Est : Droite)", "entry_angle": 180.0, "exit_angle": 90.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Nord->Sud : Tout Droit)", "entry_angle": 0.0, "exit_angle": 180.0, "arrival_time": 0.0}
            ],
            "authorized": [0, 1]
        }),
        json!({
            "id": 8,
            "name": "4 Voies (Tout Droit)",
            "vehicles": [
                {"id": 0, "name": "V0 (Ouest->Est)", "entry_angle": 270.0, "exit_angle": 90.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Nord->Sud)", "entry_angle": 0.0, "exit_angle": 180.0, "arrival_time": 0.0},
                {"id": 2, "name": "V2 (Est->Ouest)", "entry_angle": 90.0, "exit_angle": 270.0, "arrival_time": 0.0},
                {"id": 3, "name": "V3 (Sud->Nord)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 0.0}
            ],
            "authorized": [0]
        }),
        json!({
            "id": 9,
            "name": "Insertion sous traffic (Interblocage partiel)",
            "vehicles": [
                {"id": 0, "name": "V0 (Ouest->Est)", "entry_angle": 270.0, "exit_angle": 90.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Nord->Sud)", "entry_angle": 0.0, "exit_angle": 180.0, "arrival_time": 0.0},
                {"id": 2, "name": "V2 (Sud->Nord)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 0.0}
            ],
            "authorized": [0]
        }),
        json!({
            "id": 10,
            "name": "Tourne à gauche multiple",
             "vehicles": [
                {"id": 0, "name": "V0 (Sud->Ouest)", "entry_angle": 180.0, "exit_angle": 270.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Nord->Est)", "entry_angle": 0.0, "exit_angle": 90.0, "arrival_time": 0.0}
            ],
            "authorized": []
        }),
        json!({
            "id": 11,
            "name": "Interblocage Circulaire (4 Gauches)",
            "vehicles": [
                {"id": 0, "name": "V0 (Sud->Ouest)", "entry_angle": 180.0, "exit_angle": 270.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Ouest->Nord)", "entry_angle": 270.0, "exit_angle": 0.0, "arrival_time": 0.0},
                {"id": 2, "name": "V2 (Nord->Est)", "entry_angle": 0.0, "exit_angle": 90.0, "arrival_time": 0.0},
                {"id": 3, "name": "V3 (Est->Sud)", "entry_angle": 90.0, "exit_angle": 180.0, "arrival_time": 0.0}
            ],
            "authorized": []
        }),
        json!({
            "id": 12,
            "name": "6 Voies (Hexagone) - Croisement Central",
            "vehicles": [
                {"id": 0, "name": "V0 (0°->180°)", "entry_angle": 0.0, "exit_angle": 180.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (60°->240°)", "entry_angle": 60.0, "exit_angle": 240.0, "arrival_time": 0.0},
                {"id": 2, "name": "V2 (120°->300°)", "entry_angle": 120.0, "exit_angle": 300.0, "arrival_time": 0.0}
            ],
            "authorized": []
        }),
        json!({
            "id": 13,
            "name": "5 Voies - Conflit Complexe",
            "vehicles": [
                {"id": 0, "name": "V0 (0°->144°)", "entry_angle": 0.0, "exit_angle": 144.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (72°->216°)", "entry_angle": 72.0, "exit_angle": 216.0, "arrival_time": 0.0},
                {"id": 2, "name": "V2 (216°->288°)", "entry_angle": 216.0, "exit_angle": 288.0, "arrival_time": 0.0}
            ],
            "authorized": []
        }),
        json!({
            "id": 14,
            "name": "Validation Stop : V1(Stop) vs V0(Prio)",
            "vehicles": [
                {"id": 0, "name": "V0 (Sud->Nord)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Ouest->Est) AVEC STOP", "entry_angle": 270.0, "exit_angle": 90.0, "arrival_time": 0.0, "rule": "stop"}
            ],
            "authorized": []
        }),
        json!({
            "id": 15,
            "name": "Validation Cédez-le-Passage : V1(Cédez) vs V0(Prio)",
            "vehicles": [
                {"id": 0, "name": "V0 (Sud->Nord)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Ouest->Est) AVEC CEDEZ", "entry_angle": 270.0, "exit_angle": 90.0, "arrival_time": 0.0, "rule": "yield"}
            ],
            "authorized": []
        }),
        json!({
            "id": 16,
            "name": "Rond-Point: Insertion Sud (Cédez)",
            "vehicles": [
                {"id": 0, "name": "V0 (Sur l'anneau : Ouest->Est)", "entry_angle": 270.0, "exit_angle": 90.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Entrant : Sud->Est) AVEC CEDEZ", "entry_angle": 180.0, "exit_angle": 90.0, "arrival_time": 0.1, "rule": "yield"}
            ],
            "authorized": [0]
        }),
        json!({
            "id": 17,
            "name": "Rond-Point: Sortie Nord",
            "vehicles": [
                {"id": 0, "name": "V0 (Sur l'anneau : Ouest->Nord)", "entry_angle": 270.0, "exit_angle": 0.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Sur l'anneau : Ouest->Est)", "entry_angle": 270.0, "exit_angle": 90.0, "arrival_time": 2.0}
            ],
            "authorized": [0, 1]
        })
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundabout_structure() {
        let map = create_roundabout_map();
        
        // Assertions
        // 4 branches + 4 nodes on the ring = 8 intersections total
        assert_eq!(map.graph.node_count(), 8);

        // Check loops
        // Each ring node has:
        // - 1 incoming from previous ring node
        // - 1 outgoing to next ring node
        // - 1 incoming from external branch
        // - 1 outgoing to external branch
        // Total = 4 edges per ring node.
        // 4 ring nodes * 4 edges = 16 edges. (Directed)
        // External nodes have 2 edges (1 in, 1 out).
        // BUT map functions add distinct Road objects for edges.
        
        // Let's verify we have a ring. 
        // We can check if we can traverse the ring.
    }

    #[test]
    fn test_roundabout_yield_rules() {
        let map = create_roundabout_map();

        // Check that entries have Yield rules
        for node_idx in map.graph.node_indices() {
             let node = &map.graph[node_idx];
             if node.name.contains("Node") { // It's a ring node
                 // Should have a rule
                 assert!(!node.rules.is_empty(), "Ring node {} should have rules", node.name);
                 
                 // Verify the rule is Yield for the incoming external road
                 // We need to find which incoming edge is the external one.
                 // The builder assigns IDs: 
                 // Ring road: id * 10000 + i
                 // Incoming road: id * 20000 + i
                 // Outgoing road: id * 30000 + i
                 
                 // Let's just check if ANY rule is Yield
                 let _has_yield = node.rules.values().any(|r| matches!(r, RoadRule::Yield));
                 // Note: Matches! macro or just equality if derived
                 let has_yield_cmp = node.rules.values().any(|r| *r == RoadRule::Yield);
                 assert!(has_yield_cmp, "Ring node {} should have a Yield rule", node.name);
             }
        }
    }

    #[test]
    fn test_gyratory_roundabout_rules() {
        let map = create_gyratory_roundabout_map();

        // Check that NO entries have Yield or Stop rules (everything is Priority)
        // And check that we have entries in the rules map (set to Priority)
        for node_idx in map.graph.node_indices() {
             let node = &map.graph[node_idx];
             if node.name.contains("Node") { // It's a ring node
                 assert!(!node.rules.is_empty(), "Ring node {} should have rules", node.name);

                 // Verify NO Yield
                 let has_yield = node.rules.values().any(|r| *r == RoadRule::Yield);
                 assert!(!has_yield, "Gyratory node {} should NOT have a Yield rule", node.name);

                 // Verify NO Stop
                 let has_stop = node.rules.values().any(|r| *r == RoadRule::Stop);
                 assert!(!has_stop, "Gyratory node {} should NOT have a Stop rule", node.name);

                 // Verify Priority exist
                 let has_priority = node.rules.values().any(|r| *r == RoadRule::Priority);
                 assert!(has_priority, "Gyratory node {} should have Priority rules", node.name);
             }
        }
    }
}

