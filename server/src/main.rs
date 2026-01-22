mod api;
mod map;
mod simulation;

use axum::serve;
use axum::{extract::ws::WebSocketUpgrade, extract::State, routing::get, Json, Router};
use tokio::net::TcpListener;

use tower_http::cors::{Any, CorsLayer};

use crate::api::server::websocket_loop;
use crate::map::intersection::{Intersection, IntersectionKind};
use crate::map::model::Map;
use crate::map::road::{Road, RoadType};
use crate::simulation::config::SimulationConfig;
use crate::simulation::engine::Engine;
use crate::simulation::handle::Handle;
use crate::simulation::vehicle::{
    ShortestPathStrategy, TripRequest, Vehicle, VehicleKind, VehicleSpec,
};

async fn get_map(State(handle): State<Handle>) -> Json<serde_json::Value> {
    Json(handle.snapshot_map())
}

#[tokio::main]
async fn main() {
    println!("\n========== BACKEND START ==========\n");

    let mut map = Map::new();

    // Intersections (formerly Nodes)
    let h1 = map.add_intersection(Intersection {
        id: 1,
        kind: IntersectionKind::Habitation,
        name: "H1".into(),
        x: 100.0,
        y: 0.0,
    });

    let h2 = map.add_intersection(Intersection {
        id: 2,
        kind: IntersectionKind::Habitation,
        name: "H2".into(),
        x: 100.0,
        y: 200.0,
    });

    let h3 = map.add_intersection(Intersection {
        id: 5,
        kind: IntersectionKind::Habitation,
        name: "H3".into(),
        x: 100.0,
        y: 400.0,
    });

    // Workplace
    let workplace = map.add_intersection(Intersection {
        id: 3,
        kind: IntersectionKind::Workplace,
        name: "Workplace".into(),
        x: 700.0,
        y: 100.0,
    });

    // Intersection
    let intersection = map.add_intersection(Intersection {
        id: 4,
        kind: IntersectionKind::Intersection,
        name: "Intersection".into(),
        x: 400.0,
        y: 100.0,
    });

    println!("[MAP] H1 = ({}, {})", map.graph[h1].x, map.graph[h1].y);
    println!("[MAP] H2 = ({}, {})", map.graph[h2].x, map.graph[h2].y);
    println!("[MAP] H3 = ({}, {})", map.graph[h3].x, map.graph[h3].y);
    println!(
        "[MAP] Workplace = ({}, {})",
        map.graph[workplace].x, map.graph[workplace].y
    );
    println!(
        "[MAP] Intersection = ({}, {})",
        map.graph[intersection].x, map.graph[intersection].y
    );

    // Roads
    map.add_road(
        h1,
        intersection,
        Road {
            id: 1,
            road_type: RoadType::Bilateral,
            lanes: 1,
            max_speed_kmh: 30.0,
            length_m: 300.0,
            is_blocked: false,
        },
    );

    map.add_road(
        h2,
        intersection,
        Road {
            id: 2,
            road_type: RoadType::Bilateral,
            lanes: 1,
            max_speed_kmh: 30.0,
            length_m: 300.0,
            is_blocked: false,
        },
    );

    map.add_road(
        h3,
        intersection,
        Road {
            id: 4,
            road_type: RoadType::Bilateral,
            lanes: 1,
            max_speed_kmh: 30.0,
            length_m: 300.0,
            is_blocked: false,
        },
    );

    map.add_road(
        intersection,
        workplace,
        Road {
            id: 3,
            road_type: RoadType::Bilateral,
            lanes: 1,
            max_speed_kmh: 30.0,
            length_m: 300.0,
            is_blocked: false,
        },
    );

    let handle = Handle::new();
    handle.set_map(map.to_json());

    let spec_car = VehicleSpec {
        kind: VehicleKind::Car,
        max_speed_kmh: 30.0,
        length_m: 4.0,
        fuel_consumption_l_per_100km: 6.0,
        co2_g_per_km: 120.0,
    };

    let mut vehicles = Vec::new();

    // Vehicle 1: H1 -> Workplace
    let trip1 = TripRequest {
        origin_id: 1,
        destination_id: 3,
        departure_time_s: 0,
        return_time_s: None,
    };

    let vehicle1 = Vehicle::new(
        0,
        spec_car.clone(),
        trip1,
        h1,
        0,
        map.graph[h1].x,
        map.graph[h1].y,
    );

    println!(
        "[VEHICLE INIT] Vehicle {} (H1 -> Workplace) initial pos = ({}, {})",
        vehicle1.id, vehicle1.x, vehicle1.y
    );

    vehicles.push(vehicle1);

    // Vehicle 2: H2 -> Workplace
    let trip2 = TripRequest {
        origin_id: 2,
        destination_id: 3,
        departure_time_s: 0,
        return_time_s: None,
    };

    let vehicle2 = Vehicle::new(
        1,
        spec_car.clone(),
        trip2,
        h2,
        0,
        map.graph[h2].x,
        map.graph[h2].y,
    );

    println!(
        "[VEHICLE INIT] Vehicle {} (H2 -> Workplace) initial pos = ({}, {})",
        vehicle2.id, vehicle2.x, vehicle2.y
    );

    vehicles.push(vehicle2);

    // Vehicle 3: H3 -> Workplace
    let trip3 = TripRequest {
        origin_id: 5,
        destination_id: 3,
        departure_time_s: 0,
        return_time_s: None,
    };

    let vehicle3 = Vehicle::new(
        2,
        spec_car.clone(),
        trip3,
        h3,
        0,
        map.graph[h3].x,
        map.graph[h3].y,
    );

    println!(
        "[VEHICLE INIT] Vehicle {} (H3 -> Workplace) initial pos = ({}, {})",
        vehicle3.id, vehicle3.x, vehicle3.y
    );

    vehicles.push(vehicle3);

    let config = SimulationConfig {
        start_time_s: 0.0,
        end_time_s: 100.0,
        time_step_s: 0.02,
    };

    let handle_clone = handle.clone();

    let app = Router::new()
        .route(
            "/ws",
            get(move |ws: WebSocketUpgrade| {
                let handle = handle_clone.clone();
                let map_clone = map.clone();
                let vehicles_clone = vehicles.clone();
                let config_clone = config.clone();

                async move {
                    ws.on_upgrade(move |socket| async move {
                        println!("\n========== WS CONNECTED — STARTING SIM ==========\n");

                        let mut engine = Engine::new(
                            map_clone,
                            vehicles_clone,
                            config_clone,
                            ShortestPathStrategy,
                            handle.clone(),
                        );

                        tokio::spawn(async move {
                            engine.run();
                        });

                        websocket_loop(socket, handle).await;
                    })
                }
            }),
        )
        .route("/map", get(get_map)) // Renamed from /network
        .with_state(handle.clone())
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    let listener = TcpListener::bind("0.0.0.0:3014").await.unwrap();
    serve(listener, app).await.unwrap();
}
