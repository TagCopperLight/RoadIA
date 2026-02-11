mod api;
mod map;
mod simulation;

use axum::{
    extract::{
        ws::WebSocketUpgrade,
        State,
    },
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use tower_http::cors::CorsLayer;
use serde_json::json;

use crate::{
    api::server::websocket_loop,
    map::{
        intersection::{Intersection, IntersectionKind, RoadRule},
        model::Map,
    },
    simulation::handle::Handle,
};

#[derive(Clone)]
struct AppState {
    map: Map,
    handle: Handle,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // UTILISATION DE LA CARTE DE TEST (ROND-POINT)
    let map = crate::map::tests::create_roundabout_map();
    let handle = Handle::new();

    let state = AppState { map, handle: handle.clone() };

    let app = Router::new()
        .route("/", get(index))
        .route("/api/intersection-tests", get(intersection_tests_json))
        .route("/api/simple-scenario", get(simple_scenario_json))
        .route("/api/solve-scenario", post(solve_scenario))
        .route("/ws", get(ws_handler))
        .with_state(state)
        .layer(CorsLayer::permissive());

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn index() -> impl IntoResponse {
    Json(json!({"message": "API Roadia - Routes disponibles: /api/intersection-tests (JSON), /ws (WebSocket)"}))
}

async fn intersection_tests_json() -> Json<serde_json::Value> {
    let scenarios = crate::map::tests::get_test_scenarios();
    Json(json!({ "scenarios": scenarios }))
}



async fn solve_scenario(Json(payload): Json<serde_json::Value>) -> Json<serde_json::Value> {
    use crate::simulation::config::SimulationConfig;
    use crate::simulation::engine::Simulation;
    use crate::simulation::vehicle::{Vehicle, VehicleSpec, VehicleKind, TripRequest, VehicleState};
    use crate::map::model::Map;
    use petgraph::graph::NodeIndex;

    // 1. Choisir la carte
    let map_type_str = payload.get("map_type").and_then(|v| v.as_str());
    let vehicles_array = payload.get("vehicles").and_then(|v| v.as_array()).cloned().unwrap_or_default();

    let (mut map, is_roundabout) = if let Some(t) = map_type_str {
        match t {
             "gyratory" => (crate::map::tests::create_gyratory_roundabout_map(), true),
             "roundabout" => (crate::map::tests::create_roundabout_map(), true),
             "traffic_light" => (crate::map::tests::create_traffic_light_map(), false),
             _ => (crate::map::tests::create_standard_intersection_map(), false),
        }
    } else {
        // Auto-détection legacy
        let is_roundabout_legacy = vehicles_array.iter().any(|v| {
            let angle = v.get("entry_angle").and_then(|x| x.as_f64()).unwrap_or(0.0);
            (angle % 90.0).abs() > 0.1
        });
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
    for (_i, v_json) in vehicles_array.iter().enumerate() {
        let id = v_json.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
        let entry_angle = v_json.get("entry_angle").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let exit_angle = v_json.get("exit_angle").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let arrival_time = v_json.get("arrival_time").and_then(|v| v.as_f64()).map(|x| x as f32).unwrap_or(0.0);
        let rule_str_opt = v_json.get("rule").and_then(|v| v.as_str());

        // Angle entrée : on vient DE cet angle. Donc si entry_angle=180 (Sud), on part du Sud pour aller au Nord.
        // Le noeud source est donc celui à 180°.
        let entry_node = find_node(&map, entry_angle).unwrap_or(NodeIndex::new(0));
        let exit_node = find_node(&map, exit_angle).unwrap_or(NodeIndex::new(0));

        let current_time = arrival_time;
        
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
        if let Some(rule_str) = rule_str_opt {
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
            id,
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
        if let Some(rule_str) = rule_str_opt {
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
                     "lights": node.traffic_lights.iter().map(|(id, color)| (*id, format!("{:?}", color))).collect::<std::collections::HashMap<_, _>>()
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
    ws.on_upgrade(move |socket| websocket_loop(socket, state.handle, state.map))
}

// Old handle_socket removed as we now delegate to api::server::websocket_loop

