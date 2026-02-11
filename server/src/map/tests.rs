use crate::map::model::Map;
use crate::map::intersection::{Intersection, IntersectionKind, Roundabout, RoundaboutKind};
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

    // 3. Define the connections
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

pub fn create_traffic_light_map() -> Map {
    let mut map = Map::new();

    let inter = map.add_intersection(Intersection::new(1, IntersectionKind::TrafficLight, "TrafficLight".to_string(), 0.0, 0.0));
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
            "map_type": "roundabout",
            "vehicles": [
                { "id": 0, "name": "V0 (Sud->Nord)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 0.0 }
            ],
            "authorized": [0]
        }),
        json!({
            "id": 12,
            "name": "Rond-Point : Conflit Entrée vs Anneau",
            "map_type": "roundabout",
            "vehicles": [
                // V0 déjà sur l'anneau (simulé par entry_angle exotique ? non, on simule start)
                // On triche : V0 part de Ouest->Sud (passe devant le Sud)
                { "id": 0, "name": "V0 (Ouest->Sud)", "entry_angle": 270.0, "exit_angle": 180.0, "arrival_time": 0.0 },
                // V1 veut entrer au Sud (180->0) au même moment
                { "id": 1, "name": "V1 (Sud->Nord)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 2.0 } 
                // V0 arrive devant Sud vers t=2s ? A calibrer.
            ],
            "authorized": [0]
        }),
        json!({
            "id": 13,
            "name": "Carrefour Giratoire (Prio Droite) : Entrant vs Anneau",
            "map_type": "gyratory",
            "vehicles": [
                // V0 sur l'anneau (Ouest -> Sud, passe devant Sud)
                { "id": 0, "name": "V0 (Anneau)", "entry_angle": 270.0, "exit_angle": 180.0, "arrival_time": 0.0 },
                // V1 veut entrer au Sud (180 -> 0)
                // En giratoire (prio droite), V1 (Entrant) est prioritaire sur V0 (Anneau) qui vient de sa gauche
                { "id": 1, "name": "V1 (Entrant)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 0.0 }
            ],
            // V1 doit passer car prio droite. V0 doit attendre.
            "authorized": [1] 
        }),
        json!({
            "id": 14,
            "name": "COMPARATIF 1/2 : Rond-Point Classique (Cédez à l'entrée)",
            "map_type": "roundabout", // CEDEZ LE PASSAGE pour l'entrant
            "vehicles": [
                // V0 part de l'Ouest, doit parcourir Ouest->Sud sur l'anneau. 
                // Distance ~120m. Temps ~12s.
                { "id": 0, "name": "V0 (Anneau)", "entry_angle": 270.0, "exit_angle": 90.0, "arrival_time": 0.0 },
                // V1 part du Sud. Distance ~60m. Temps ~4-5s.
                // On retarde V1 de 7.5s pour qu'il arrive au cédez-le-passage EN MEME TEMPS que V0.
                { "id": 1, "name": "V1 (Entrant)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 7.5 }
            ],
            // Règle Rond-point : Celui sur l'anneau est prioritaire. V1 freine.
            "authorized": [0]
        }),
        json!({
            "id": 15,
            "name": "COMPARATIF 2/2 : Giratoire (Priorité à Droite)",
            "map_type": "gyratory", // PRIORITE A DROITE (Entrant est à droite)
            "vehicles": [
                // Même configuration temporelle (Synchronisation au point de conflit)
                { "id": 0, "name": "V0 (Anneau)", "entry_angle": 270.0, "exit_angle": 90.0, "arrival_time": 0.0 },
                { "id": 1, "name": "V1 (Entrant)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 7.5 }
            ],
            // Règle Prio Droite : L'entrant vient de droite. V0 freine sur l'anneau.
            "authorized": [1]
        }),
        json!({
            "id": 16,
            "name": "Validation Feux Tricolores",
            "map_type": "traffic_light",
            "vehicles": [
                {"id": 0, "name": "V0 (Nord->Sud)", "entry_angle": 0.0, "exit_angle": 180.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Est->Ouest)", "entry_angle": 90.0, "exit_angle": 270.0, "arrival_time": 0.0}
            ],
            "description": "Feux tricolores actifs."
        })
    ]
}
