use crate::map::model::Map;
use crate::map::intersection::{Intersection, IntersectionKind, Roundabout, RoadRule};
use crate::map::road::Road;
use std::collections::HashMap;
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
        40.0 // Rayon de 40m pour correspondre à environ R_RING_OUTER visualisé
    );

    // 2. Create external intersections (inputs/outputs)
    let north = map.add_intersection(Intersection {
        id: 101,
        kind: IntersectionKind::Habitation,
        name: "North".to_string(),
        x: 0.0,
        y: 100.0,
        rules: HashMap::new(),
    });

    let east = map.add_intersection(Intersection {
        id: 102,
        kind: IntersectionKind::Habitation,
        name: "East".to_string(),
        x: 100.0,
        y: 0.0,
        rules: HashMap::new(),
    });

    let south = map.add_intersection(Intersection {
        id: 103,
        kind: IntersectionKind::Habitation,
        name: "South".to_string(),
        x: 0.0,
        y: -100.0,
        rules: HashMap::new(),
    });

    let west = map.add_intersection(Intersection {
        id: 104,
        kind: IntersectionKind::Habitation,
        name: "West".to_string(),
        x: -100.0,
        y: 0.0,
        rules: HashMap::new(),
    });

    // 3. Define the connections
    let connections = vec![north, east, south, west];

    // 4. Build the roundabout in the map
    let _ring_nodes = roundabout.build(&mut map, connections);

    map
}

pub fn create_standard_intersection_map() -> Map {
    let mut map = Map::new();

    let inter = map.add_intersection(Intersection {
        id: 1,
        kind: IntersectionKind::Intersection,
        name: "Intersection".to_string(),
        x: 0.0,
        y: 0.0,
        rules: HashMap::new(),
    });

    let h_north = map.add_intersection(Intersection {
        id: 2,
        kind: IntersectionKind::Habitation,
        name: "H-North".to_string(),
        x: 0.0,
        y: 100.0,
        rules: HashMap::new(),
    });

    let h_east = map.add_intersection(Intersection {
        id: 3,
        kind: IntersectionKind::Habitation,
        name: "H-East".to_string(),
        x: 100.0,
        y: 0.0,
        rules: HashMap::new(),
    });

    let h_south = map.add_intersection(Intersection {
        id: 4,
        kind: IntersectionKind::Habitation,
        name: "H-South".to_string(),
        x: 0.0,
        y: -100.0,
        rules: HashMap::new(),
    });
    
    let h_west = map.add_intersection(Intersection {
        id: 5,
        kind: IntersectionKind::Habitation,
        name: "H-West".to_string(),
        x: -100.0,
        y: 0.0,
        rules: HashMap::new(),
    });

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
           "name": "Stop vs Priority",
           "vehicles": [
               { "id": 0, "name": "V0 (Sud->Nord) Stop", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 0.0, "rule": "stop" },
               { "id": 1, "name": "V1 (Ouest->Est) Prio", "entry_angle": 270.0, "exit_angle": 90.0, "arrival_time": 0.0, "rule": "priority" }
           ],
           "authorized": [1]
        }),
        json!({
            "id": 10,
            "name": "Yield vs Priority",
            "vehicles": [
                { "id": 0, "name": "V0 (Sud->Nord) Yield", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 0.0, "rule": "yield" },
                { "id": 1, "name": "V1 (Ouest->Est) Prio", "entry_angle": 270.0, "exit_angle": 90.0, "arrival_time": 0.0, "rule": "priority" }
            ],
            "authorized": [1]
        }),
        json!({
            "id": 11,
            "name": "Rond-Point : Entrée Libre",
            "vehicles": [
                { "id": 0, "name": "V0 (Sud->Nord)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 0.0 }
            ],
            "authorized": [0]
        }),
        json!({
            "id": 12,
            "name": "Rond-Point : Conflit Entrée vs Anneau",
            "vehicles": [
                // V0 déjà sur l'anneau (simulé par entry_angle exotique ? non, on simule start)
                // On triche : V0 part de Ouest->Sud (passe devant le Sud)
                { "id": 0, "name": "V0 (Ouest->Sud)", "entry_angle": 270.0, "exit_angle": 180.0, "arrival_time": 0.0 },
                // V1 veut entrer au Sud (180->0) au même moment
                { "id": 1, "name": "V1 (Sud->Nord)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 2.0 } 
                // V0 arrive devant Sud vers t=2s ? A calibrer.
            ],
            "authorized": [0]
        })
    ]
}
