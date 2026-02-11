mod api;
mod map;
mod simulation;

use std::time::Duration;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::{Html, IntoResponse, Json},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use serde_json::json;

use crate::{
    map::{
        intersection::{Intersection, IntersectionKind, RoadRule},
        model::Map,
    },
    simulation::{
        config::SimulationConfig,
        engine::Simulation,
        vehicle::{fastest_path, TripRequest, Vehicle, VehicleKind, VehicleSpec, VehicleState},
    },
};

#[derive(Clone)]
struct AppState {
    map: Map,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // UTILISATION DE LA CARTE DE TEST (ROND-POINT)
    let map = crate::map::tests::create_roundabout_map();

    let state = AppState { map };

    let app = Router::new()
        .route("/", get(index))
        .route("/intersection-dynamic", get(intersection_dynamic))
        .route("/api/intersection-tests", get(intersection_tests_json))
        .route("/api/simple-scenario", get(simple_scenario_json))
        .route("/api/solve-scenario", post(solve_scenario))
        .route("/ws", get(ws_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> impl IntoResponse {
    Json(json!({"message": "API Roadia - Routes disponibles: /intersection-dynamic (tests), /api/intersection-tests (JSON), /ws (WebSocket)"}))
}

async fn intersection_dynamic() -> impl IntoResponse {
    Html(include_str!("../static/intersection_dynamic.html"))
}

async fn intersection_tests_json() -> Json<serde_json::Value> {
    let scenarios = crate::map::tests::get_test_scenarios();
    Json(json!({ "scenarios": scenarios }))
}



#[derive(Debug, Deserialize)]
struct TestVehicle {
    id: u64,
    #[serde(default)]
    name: String,
    entry_angle: f64,
    exit_angle: f64,
    arrival_time: f32,
    
    // --- CHAMP AJOUTÉ POUR LE CONTRÔLE DES RÈGLES ---
    // Permet de spécifier explicitement la priorité du véhicule
    // Valeurs acceptées: "Stop", "Yield" (Cédez), "Priority"
    // Si None, la règle par défaut de la carte s'applique
    #[serde(default)]
    rule: Option<String>, 
}

#[derive(Debug, Deserialize)]
struct SolveRequest {
    vehicles: Vec<TestVehicle>,
    #[serde(default)]
    map_type: Option<String>,
}

async fn solve_scenario(Json(payload): Json<SolveRequest>) -> Json<serde_json::Value> {
    use crate::simulation::config::SimulationConfig;
    use crate::simulation::engine::Simulation;
    use crate::simulation::vehicle::{Vehicle, VehicleSpec, VehicleKind, TripRequest, VehicleState};
    use crate::map::model::Map;
    use petgraph::graph::NodeIndex;

    // 1. Choisir la carte
    let (mut map, is_roundabout) = if let Some(t) = &payload.map_type {
        match t.as_str() {
             "gyratory" => (crate::map::tests::create_gyratory_roundabout_map(), true),
             "roundabout" => (crate::map::tests::create_roundabout_map(), true),
             "traffic_light" => (crate::map::tests::create_traffic_light_map(), false),
             _ => (crate::map::tests::create_standard_intersection_map(), false),
        }
    } else {
        // Auto-détection legacy
        let is_roundabout_legacy = payload.vehicles.iter().any(|v| (v.entry_angle % 90.0).abs() > 0.1);
        if is_roundabout_legacy {
            (crate::map::tests::create_roundabout_map(), true)
        } else {
            (crate::map::tests::create_standard_intersection_map(), false)
        }
    };

    // 3. Helper pour trouver les noeuds (Entrée/Sortie)
    let find_node = |m: &Map, angle_deg: f64| -> Option<NodeIndex> {
        // Logique de conversion Angle -> Position sur la carte
        // Convention : 
        // 0° (Nord) -> x=0, y=100
        // 90° (Est) -> x=100, y=0
        // 180° (Sud) -> x=0, y=-100
        // 270° (Ouest) -> x=-100, y=0
        let mut best_node = None;
        let mut min_dist = f32::MAX;
        
        // On convertit l'angle "Trigo standard" vers coordonnées cartésiennes
        // Dans le modèle Test : 0° est Nord (y+), Sens Horaire ?
        // Vérif via `create_standard_intersection_map` :
        // North (id 2) : (0, 100)
        // East (id 3) : (100, 0)
        // South (id 4) : (0, -100)
        // West (id 5) : (-100, 0)
        
        let target_x = 100.0 * (angle_deg.to_radians().sin() as f32); // 0->0, 90->1
        let target_y = 100.0 * (angle_deg.to_radians().cos() as f32); // 0->1, 90->0
        
        for idx in m.graph.node_indices() {
             let node = &m.graph[idx];
             // Filtre noeuds internes
             if node.name.contains("Node-") 
                || node.name == "Intersection" 
                || node.name.contains("RondPoint") { 
                 continue; 
             }
             
             let dist = ((node.x - target_x).powi(2) + (node.y - target_y).powi(2)).sqrt();
             if dist < min_dist {
                 min_dist = dist;
                 best_node = Some(idx);
             }
        }
        best_node
    };

    // 4. Préparer les véhicules
    let mut pending_vehicles: Vec<(f32, Vehicle)> = Vec::new();
    
    // Pour chaque véhicule, on configure le chemin ET les règles de priorité (stop, céder le passage...)
    for (_i, v_req) in payload.vehicles.iter().enumerate() {
        // Angle entrée : on vient DE cet angle. Donc si entry_angle=180 (Sud), on part du Sud pour aller au Nord.
        // Le noeud source est donc celui à 180°.
        let entry_node = find_node(&map, v_req.entry_angle).unwrap_or(NodeIndex::new(0));
        let exit_node = find_node(&map, v_req.exit_angle).unwrap_or(NodeIndex::new(0));

        let current_time = v_req.arrival_time;
        
        let spec = VehicleSpec {
            kind: VehicleKind::Car,
            max_speed_ms: if is_roundabout { 8.33 } else { 13.8 },
            max_acceleration_ms2: 3.0,
            comfortable_deceleration: 2.0,
            reaction_time: 1.0,
            length_m: 4.5,
            fuel_consumption_l_per_100km: 5.0,
            co2_g_per_km: 100.0,
        };

        let path = crate::simulation::vehicle::fastest_path(&map, entry_node, exit_node);
        
        if path.len() < 2 { continue; }

        // --- GESTION DES RÈGLES DE PRIORITÉ (Payload JSON) ---
        // Si le champ 'rule' est présent dans le JSON pour ce véhicule
        if let Some(rule_str) = &v_req.rule {
            use crate::map::intersection::RoadRule;
            
            // On récupère le nœud source (début de la route)
            let source_node = path[0];
            // On récupère le nœud d'intersection (fin de la route)
            let intersection_node = path[1];
            
            // On cherche l'identifiant de la route (Edge) reliant ces deux nœuds
            if let Some(edge_idx) = map.graph.find_edge(source_node, intersection_node) {
                let road_id = map.graph[edge_idx].id; // ID unique de la route
                
                // Normalisation de la chaîne en minuscules pour éviter les erreurs de casse
                let rule_str_lower = rule_str.to_lowercase();
                
                // Conversion de la string JSON en Enum Rust (RoadRule)
                let parsed_rule = match rule_str_lower.as_str() {
                    "stop" => RoadRule::Stop,                   // Arrêt obligatoire
                    "yield" | "let_passage" => RoadRule::Yield, // Cédez le passage
                    "priority" => RoadRule::Priority,           // Route prioritaire
                    _ => RoadRule::Priority,                    // Par défaut
                };
                
                // Mise à jour de la configuration de l'intersection dans la carte locale
                // On associe la règle parsée à l'identifiant de la route entrante
                if let Some(inter) = map.graph.node_weight_mut(intersection_node) {
                    inter.rules.insert(road_id, parsed_rule);
                }
            }
        }

        let mut vehicle = Vehicle::new(
            v_req.id,
            spec,
            TripRequest { origin_id: 0, destination_id: 0, departure_time_s: 0, return_time_s: None },
            entry_node,
        );
        vehicle.path = path.clone();
        if vehicle.path.len() > 1 {
            vehicle.next_node = Some(vehicle.path[1]);
        }

        // --- INJECTION DE RÈGLES FORCÉES (Niveau Véhicule) ---
        // Cette section redondante assure que le véhicule porte lui-même la règle
        // C'est utile pour le moteur de simulation qui vérifie 'vehicle.forced_rules'
        if let Some(rule_str) = &v_req.rule {
            use crate::map::intersection::RoadRule;
            // Normalisation à nouveau (pour être sûr)
            let rule_lower = rule_str.to_lowercase();
            // Parsing identique au bloc précédent
            let parsed_rule = match rule_lower.as_str() {
                "stop" => RoadRule::Stop,
                "yield" | "let_passage" => RoadRule::Yield,
                "priority" => RoadRule::Priority,
                _ => {
                   // Log d'avertissement si la règle est inconnue
                   println!("Attention: Règle inconnue '{}', par défaut Priority", rule_str);
                   RoadRule::Priority
                }
            };
            
            // On applique cette règle forcée à la prochaine intersection du chemin
            // Cela garantit que le moteur de simulation (engine.rs) respectera ce choix
            if path.len() > 1 {
                 vehicle.forced_rules.insert(path[1], parsed_rule);
            }
        }
        
        pending_vehicles.push((current_time, vehicle));
    }

    
    pending_vehicles.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

    // 5. Initialiser la simulation
    let mut sim = SimulationConfig::new(
        map, 0.0, 60.0, 0.1, Vec::new(), 2.5, 4.0,
    );

    // 6. Boucle de Simulation
    let mut frames = Vec::new();
    let steps = (60.0 / 0.1) as usize;

    for _step in 0..steps {
        let time = sim.current_time;

        // Injection
        while !pending_vehicles.is_empty() {
            if pending_vehicles[0].0 <= time {
                let (_, v) = pending_vehicles.remove(0);
                sim.vehicles.push(v);
            } else {
                break;
            }
        }

        sim.step();

        let frame_data: Vec<serde_json::Value> = sim.vehicles.iter().map(|v| {
            let coords = v.get_coordinates(&sim.map);
            let mut angle = 0.0;
            // Calcul approximatif de l'angle pour affichage via vitesse
            if v.velocity > 0.1 {
                 // On utilise v.current_edge_progress si possible, mais ici on SIMPLIFIE
                 // Si on garde la méthode précédente (dx, dy), l'angle est constant sur tout le segment
                 // Pour un affichage plus fluide sur rond-point, il faudrait interpoler les segments.
                 // Pour l'instant on garde la méthode simple.
            }
             if let (Some(curr), Some(next)) = (sim.map.graph.node_weight(v.current_node), v.next_node.and_then(|n| sim.map.graph.node_weight(n))) {
                 let dx = next.x - curr.x;
                 let dy = next.y - curr.y;
                 angle = dy.atan2(dx).to_degrees();
             }
            
            json!({
                "id": v.id,
                "x": coords.x,
                "y": coords.y,
                "angle": angle,
                "speed": v.velocity,
                "state": format!("{:?}", v.state)
            })
        }).collect();

        // Capture Traffic Light State
        let mut lights_data = Vec::new();
        for node in sim.map.graph.node_weights() {
            if !node.traffic_lights.is_empty() {
                 lights_data.push(json!({
                     "intersection_id": node.id,
                     "lights": node.traffic_lights
                 }));
            }
        }

        frames.push(json!({
            "time": time,
            "vehicles": frame_data,
            "lights": lights_data
        }));

        if sim.vehicles.iter().all(|v| v.state == VehicleState::Arrived) && pending_vehicles.is_empty() {
            break;
        }

        sim.current_time += sim.time_step_s;
    }

    Json(json!({
        "mode": "replay",
        "map_type": if is_roundabout { "roundabout" } else { "intersection" },
        "frames": frames
    }))
}

async fn simple_scenario_json() -> Json<serde_json::Value> {
    use crate::map::intersection::{JunctionController, MovementRequest};
    use petgraph::graph::NodeIndex;
    
    let center = Intersection::new(0, IntersectionKind::Intersection, "Center".to_string(), 0.0, 0.0);
    let north = Intersection::new(1, IntersectionKind::Habitation, "H-North".to_string(), 0.0, 100.0);
    let east = Intersection::new(2, IntersectionKind::Habitation, "H-East".to_string(), 100.0, 0.0);
    let south = Intersection::new(3, IntersectionKind::Habitation, "H-South".to_string(), 0.0, -100.0);
    let ldt = Intersection::new(5, IntersectionKind::Workplace, "LDT".to_string(), -100.0, 0.0);

    let north_angle = center.compute_road_angle(&north); 
    let east_angle = center.compute_road_angle(&east);   
    let south_angle = center.compute_road_angle(&south); 
    let ldt_angle = center.compute_road_angle(&ldt);     

    let all_entry_angles = vec![north_angle, east_angle, south_angle, ldt_angle];

    let requests = vec![
        MovementRequest {
            vehicle_index: 0,
            vehicle_id: 0,
            to: NodeIndex::new(5), // LDT
            entry_angle: north_angle,
            exit_angle: ldt_angle,
            arrival_time: 0.0,
            rule: RoadRule::Priority,
            light_color: None,
        },
        MovementRequest {
            vehicle_index: 1,
            vehicle_id: 1,
            to: NodeIndex::new(5),
            entry_angle: east_angle,
            exit_angle: ldt_angle,
            arrival_time: 0.0,
            rule: RoadRule::Priority,
            light_color: None,
        },
        MovementRequest {
            vehicle_index: 2,
            vehicle_id: 2,
            to: NodeIndex::new(5),
            entry_angle: south_angle,
            exit_angle: ldt_angle,
            arrival_time: 0.0,
            rule: RoadRule::Priority,
            light_color: None,
        },
    ];
    
    let authorized_indices = JunctionController::authorized_indices(&requests, &all_entry_angles);
    
    Json(json!({
        "vehicles": [
            {"id": 0, "name": "V0", "start": "H-North", "entry_angle": north_angle, "exit_angle": ldt_angle},
            {"id": 1, "name": "V1", "start": "H-East", "entry_angle": east_angle, "exit_angle": ldt_angle},
            {"id": 2, "name": "V2", "start": "H-South", "entry_angle": south_angle, "exit_angle": ldt_angle}
        ],
        "authorized": authorized_indices
    }))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let map = state.map;

    let inter = map
        .graph
        .node_indices()
        .find(|i| map.graph[*i].id == 1)
        .expect("Intersection not found");
    let ldt = map
        .graph
        .node_indices()
        .find(|i| map.graph[*i].id == 5)
        .expect("LDT not found");
    let h_north = map
        .graph
        .node_indices()
        .find(|i| map.graph[*i].id == 2)
        .expect("H-North not found");
    let h_east = map
        .graph
        .node_indices()
        .find(|i| map.graph[*i].id == 3)
        .expect("H-East not found");
    let h_south = map
        .graph
        .node_indices()
        .find(|i| map.graph[*i].id == 4)
        .expect("H-South not found");

    let spec = VehicleSpec {
        kind: VehicleKind::Car,
        max_speed_ms: 12.0,
        max_acceleration_ms2: 2.5,
        comfortable_deceleration: 0.5,
        reaction_time: 1.0,
        length_m: 4.5,
        fuel_consumption_l_per_100km: 6.0,
        co2_g_per_km: 120.0,
    };

    let mut vehicles = Vec::new();

    for (vehicle_id, (h_node, h_id)) in [(h_north, 2), (h_east, 3), (h_south, 4)].iter().enumerate()
    {
        let trip = TripRequest {
            origin_id: *h_id,
            destination_id: 5, 
            departure_time_s: 0,
            return_time_s: None,
        };

        let path = fastest_path(&map, *h_node, ldt);
        let mut vehicle = Vehicle::new((vehicle_id + 1) as u64, spec.clone(), trip, *h_node);
        vehicle.path = path.clone();
        vehicle.path_index = 0;
        vehicle.current_node = *path.first().expect("path should not be empty");
        vehicle.next_node = path.get(1).copied();
        vehicle.state = VehicleState::EnRoute;
        let edge_index = map
            .graph
            .find_edge(*h_node, inter)
            .expect("edge should exist");
        let road = map.graph.edge_weight(edge_index).expect("road weight");
        vehicle.position_on_edge_m = road.length_m;

        vehicles.push(vehicle);
    }

    let mut sim = SimulationConfig::new(map.clone(), 0.0, 120.0, 1.0, vehicles, 2.0, 4.0);

    let inter_coords = &map.graph[inter];
    let ldt_coords = &map.graph[ldt];
    let h_north_coords = &map.graph[h_north];
    let h_east_coords = &map.graph[h_east];
    let h_south_coords = &map.graph[h_south];

    let roads_data: Vec<_> = map
        .graph
        .raw_edges()
        .iter()
        .map(|edge| {
            let (from_idx, to_idx) = (edge.source(), edge.target());
            let from_id = map.graph[from_idx].id;
            let to_id = map.graph[to_idx].id;
            let road = &edge.weight;
            serde_json::json!({
                "from_id": from_id,
                "to_id": to_id,
                "length_m": road.length_m,
            })
        })
        .collect();

    let init_msg = serde_json::json!({
        "type": "init",
        "intersections": [
            {"id": 1, "name": "Intersection", "x": inter_coords.x, "y": inter_coords.y},
            {"id": 2, "name": "H-North", "x": h_north_coords.x, "y": h_north_coords.y},
            {"id": 3, "name": "H-East", "x": h_east_coords.x, "y": h_east_coords.y},
            {"id": 4, "name": "H-South", "x": h_south_coords.x, "y": h_south_coords.y},
            {"id": 5, "name": "LDT", "x": ldt_coords.x, "y": ldt_coords.y},
        ],
        "roads": roads_data
    });

    let mut socket = socket;
    if socket
        .send(Message::Text(init_msg.to_string()))
        .await
        .is_err()
    {
        return;
    }

    while sim.current_time < sim.end_time_s {
        sim.step();
        sim.current_time += sim.time_step_s;

        let snapshot = serde_json::json!({
            "type": "update",
            "time_s": sim.current_time,
            "vehicles": sim.vehicles.iter().map(|v| {
                let coords = v.get_coordinates(&sim.map);
                serde_json::json!({
                    "id": v.id,
                    "state": format!("{:?}", v.state),
                    "x": coords.x,
                    "y": coords.y,
                    "velocity": v.velocity,
                })
            }).collect::<Vec<_>>()
        });

        if socket
            .send(Message::Text(snapshot.to_string()))
            .await
            .is_err()
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis((sim.time_step_s * 1000.0) as u64)).await;
    }
}
