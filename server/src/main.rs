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
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    map::{
        intersection::{Intersection, IntersectionKind, JunctionController, MovementRequest},
        model::Map,
        road::{Road, RoadRule},
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
    let mut map = Map::new();

    let inter = map.add_intersection(Intersection {
        id: 1,
        kind: IntersectionKind::Intersection,
        name: "Intersection".to_string(),
        x: 0.0,
        y: 0.0,
    });

    let ldt = map.add_intersection(Intersection {
        id: 5,
        kind: IntersectionKind::Workplace,
        name: "LDT".to_string(),
        x: 0.0,
        y: -150.0,
    });

    let h_north = map.add_intersection(Intersection {
        id: 2,
        kind: IntersectionKind::Habitation,
        name: "H-North".to_string(),
        x: 0.0,
        y: 100.0,
    });

    let h_east = map.add_intersection(Intersection {
        id: 3,
        kind: IntersectionKind::Habitation,
        name: "H-East".to_string(),
        x: 100.0,
        y: 0.0,
    });

    let h_south = map.add_intersection(Intersection {
        id: 4,
        kind: IntersectionKind::Habitation,
        name: "H-South".to_string(),
        x: -100.0,
        y: 0.0,
    });

    map.add_two_way_road(h_north, inter, Road::new(1, 1, 12, 100.0, false, false));
    map.add_two_way_road(h_east, inter, Road::new(2, 1, 12, 100.0, false, false));
    map.add_two_way_road(h_south, inter, Road::new(3, 1, 12, 100.0, false, false));

    map.add_two_way_road(inter, ldt, Road::new(4, 1, 12, 150.0, false, false));

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
    let scenarios = vec![
        json!({
            "id": 1,
            "name": "FIFO (Tout Droit)",
            "vehicles": [
                {"id": 0, "name": "V0 (Sud->Nord)", "entry_angle": 180.0, "exit_angle": 0.0, "arrival_time": 0.0},
                {"id": 1, "name": "V1 (Nord->Sud)", "entry_angle": 0.0, "exit_angle": 180.0, "arrival_time": 5.0} // Arrive plus tard
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
        })
    ];
    
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
    #[serde(default)]
    rule: Option<String>, // "stop", "yield", "priority"
}

#[derive(Debug, Deserialize)]
struct SolveRequest {
    vehicles: Vec<TestVehicle>,
}

async fn solve_scenario(Json(payload): Json<SolveRequest>) -> Json<serde_json::Value> {
    use petgraph::graph::NodeIndex;
    use std::collections::{HashSet, HashMap};
    use crate::simulation::vehicle::{Vehicle, VehicleSpec, VehicleKind, TripRequest, VehicleState};

    let compute_crossing_duration = |distance_m: f32, initial_v: f32| -> f64 {
        let spec = VehicleSpec {
            kind: VehicleKind::Car,
            max_speed_ms: 13.8, // ~50 km/h
            max_acceleration_ms2: 3.0,
            comfortable_deceleration: 2.0,
            reaction_time: 1.0,
            length_m: 4.5,
            fuel_consumption_l_per_100km: 5.0,
            co2_g_per_km: 100.0,
        };
        
        if distance_m <= 0.001 { return 0.0; }

        let mut v = initial_v;
        let mut d = 0.0f32;
        let mut t = 0.0f64;
        let dt = 0.1;
        
        while d < distance_m {
            let acc = spec.max_acceleration_ms2 * (1.0 - (v / spec.max_speed_ms).powi(4));
            v += acc * dt as f32;
            if v < 0.0 { v = 0.0; }
            d += v * dt as f32;
            t += dt;
        }
        t
    };

    let crossing_dist_m = 40.0;

    let mut angles_set = HashSet::new();
    for v in &payload.vehicles {
        let entry = (v.entry_angle + 360.0) % 360.0;
        let exit = (v.exit_angle + 360.0) % 360.0;
        let entry_int = (entry * 10.0).round() as i64;
        let exit_int = (exit * 10.0).round() as i64;
        angles_set.insert(entry_int);
        angles_set.insert(exit_int);
    }
    let all_entry_angles: Vec<f64> = angles_set.clone().into_iter().map(|a| a as f64 / 10.0).collect();

    let angle_to_node: HashMap<i64, NodeIndex> = angles_set.into_iter()
        .enumerate()
        .map(|(i, angle)| (angle, NodeIndex::new(i)))
        .collect();

    let mut time = 0.0;
    let tick = 0.1;
    
    let mut schedule: HashMap<u64, (f64, f64, f32)> = HashMap::new(); // id -> (start, end, v_init)
    let mut finished: HashSet<u64> = HashSet::new();
    
    while finished.len() < payload.vehicles.len() && time < 60.0 {
        let mut active_crossing: Vec<usize> = Vec::new();
        
        for (i, v) in payload.vehicles.iter().enumerate() {
            if let Some(&(start, end, _)) = schedule.get(&v.id) {
                if time >= start && time < end {
                    active_crossing.push(i);
                }
                if time >= end {
                    finished.insert(v.id);
                }
            }
        }

        let mut candidates: Vec<usize> = Vec::new(); 
        for (i, v) in payload.vehicles.iter().enumerate() {
            if !schedule.contains_key(&v.id) && v.arrival_time <= (time as f32) + 0.01 {
                candidates.push(i);
            }
        }

        let mut authorized_now: Vec<usize> = Vec::new();
        
        let mut requests_mix: Vec<MovementRequest> = Vec::new();
        let mut mix_map: Vec<usize> = Vec::new(); 

        for &idx in &active_crossing {
            let v = &payload.vehicles[idx];
            let exit_int = ((v.exit_angle + 360.0) % 360.0 * 10.0).round() as i64;
            requests_mix.push(MovementRequest {
                vehicle_index: idx,
                vehicle_id: v.id,
                to: *angle_to_node.get(&exit_int).unwrap_or(&NodeIndex::new(0)),
                entry_angle: v.entry_angle, 
                exit_angle: v.exit_angle,
                arrival_time: -1.0, 
                rule: RoadRule::Priority,
            });
            mix_map.push(idx);
        }

        for &idx in &candidates {
            let v = &payload.vehicles[idx];
            
            // Determine Rule from JSON
            let rule = match v.rule.as_deref() {
                Some("stop") => RoadRule::Stop,
                Some("yield") => RoadRule::Yield,
                _ => RoadRule::Priority,
            };

            // Stop Logic Simulation
            if rule == RoadRule::Stop {
                if time - (v.arrival_time as f64) < 3.0 {
                    continue; 
                }
            }

            let exit_int = ((v.exit_angle + 360.0) % 360.0 * 10.0).round() as i64;
            requests_mix.push(MovementRequest {
                vehicle_index: idx, 
                vehicle_id: v.id,
                to: *angle_to_node.get(&exit_int).unwrap_or(&NodeIndex::new(0)),
                entry_angle: v.entry_angle,
                exit_angle: v.exit_angle,
                arrival_time: v.arrival_time,
                rule,
            });
            mix_map.push(idx);
        }

        let allowed_in_mix = JunctionController::authorized_indices(&requests_mix, &all_entry_angles);
        
        for &mix_idx in &allowed_in_mix {
             let original_idx = mix_map[mix_idx];
             if candidates.contains(&original_idx) {
                 authorized_now.push(original_idx);
             }
        }

        if active_crossing.is_empty() && !candidates.is_empty() && authorized_now.is_empty() {
            candidates.sort_by_key(|&idx| payload.vehicles[idx].id);
            if let Some(&winner_idx) = candidates.first() {
                authorized_now.push(winner_idx);
            }
        }

        if !authorized_now.is_empty() {
            for &idx in &authorized_now {
                let v = &payload.vehicles[idx];
                let vid = v.id;
                
                let waited = time > (v.arrival_time as f64 + 0.15); 
                let v_init = if waited { 0.0 } else { 13.8 }; 
                let duration = compute_crossing_duration(crossing_dist_m, v_init);

                schedule.insert(vid, (time, time + duration, v_init));
            }
        }

        time += tick;
    }

    let vehicles_resp: Vec<serde_json::Value> = payload.vehicles.iter().map(|v| {
        let (start, end, v_init) = *schedule.get(&v.id).unwrap_or(&(999.0, 999.0, 0.0));
        json!({
            "id": v.id,
            "name": if v.name.is_empty() { format!("V{}", v.id) } else { v.name.clone() },
            "entry_angle": v.entry_angle,
            "exit_angle": v.exit_angle,
            "arrival_time": v.arrival_time,
            "start_time": start,
            "end_time": end,
            "initial_velocity": v_init
        })
    }).collect();

    Json(json!({
        "vehicles": vehicles_resp,
        "debug_log": "Simulation computed server-side with Physics V2"
    }))
}

async fn simple_scenario_json() -> Json<serde_json::Value> {
    use crate::map::intersection::{JunctionController, MovementRequest};
    use petgraph::graph::NodeIndex;
    
    let center = Intersection {
        id: 0,
        kind: IntersectionKind::Intersection,
        name: "Center".to_string(),
        x: 0.0,
        y: 0.0,
    };

    let north = Intersection {
        id: 1,
        kind: IntersectionKind::Habitation,
        name: "H-North".to_string(),
        x: 0.0,
        y: 100.0, 
    };

    let east = Intersection {
        id: 2,
        kind: IntersectionKind::Habitation,
        name: "H-East".to_string(),
        x: 100.0, 
        y: 0.0,
    };

    let south = Intersection {
        id: 3,
        kind: IntersectionKind::Habitation,
        name: "H-South".to_string(),
        x: 0.0,
        y: -100.0,
    };

    let ldt = Intersection {
        id: 5,
        kind: IntersectionKind::Workplace,
        name: "LDT".to_string(),
        x: -100.0,
        y: 0.0,
    };

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
        },
        MovementRequest {
            vehicle_index: 1,
            vehicle_id: 1,
            to: NodeIndex::new(5),
            entry_angle: east_angle,
            exit_angle: ldt_angle,
            arrival_time: 0.0,
            rule: RoadRule::Priority,
        },
        MovementRequest {
            vehicle_index: 2,
            vehicle_id: 2,
            to: NodeIndex::new(5),
            entry_angle: south_angle,
            exit_angle: ldt_angle,
            arrival_time: 0.0,
            rule: RoadRule::Priority,
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
