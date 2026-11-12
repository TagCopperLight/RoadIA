//! Offline server capacity measurement; no browser or WebSocket transport.
//! Args: grid_side_nodes commute_count simulated_seconds seed.
//! Grid generation follows server/examples/stress.rs.

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use server::api::runner::map_generator::create_random_commutes_with_rng;
use server::api::websocket::{serialize_traffic_lights, serialize_vehicle, ServerPacket};
use server::map::intersection::{build_intersections, IntersectionKind};
use server::map::model::Map;
use server::simulation::config::SimulationConfig;
use server::simulation::engine::{Simulation, SimulationEngine};
use server::simulation::vehicle::VehicleState;
use std::time::Instant;

fn grid_map(n: usize, lanes: u8) -> Map {
    let mut map = Map::new();
    let spacing = 150.0;
    let mut ids = vec![vec![0u32; n]; n];
    for (y, row) in ids.iter_mut().enumerate() {
        for (x, id) in row.iter_mut().enumerate() {
            let border = x == 0 || y == 0 || x == n - 1 || y == n - 1;
            let kind = if !border {
                IntersectionKind::Intersection
            } else if (x + y) % 2 == 0 {
                IntersectionKind::Habitation
            } else {
                IntersectionKind::Workplace
            };
            *id = map.add_intersection(kind, x as f32 * spacing, y as f32 * spacing);
        }
    }
    for y in 0..n {
        for x in 0..n {
            if x + 1 < n {
                map.add_two_way_road(ids[y][x], ids[y][x + 1], lanes, 14.0, spacing - 10.0);
            }
            if y + 1 < n {
                map.add_two_way_road(ids[y][x], ids[y + 1][x], lanes, 14.0, spacing - 10.0);
            }
        }
    }
    build_intersections(&mut map);
    map
}

fn percentile(xs: &[f64], p: f64) -> f64 {
    xs[((xs.len() - 1) as f64 * p).ceil() as usize]
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    let n: usize = a[1].parse().unwrap();
    let count: usize = a[2].parse().unwrap();
    let seconds: usize = a[3].parse().unwrap();
    let seed: u64 = a[4].parse().unwrap();
    let setup = Instant::now();
    let mut map = grid_map(n, 1);
    map.settings.use_day_night_cycle = false;
    let nodes = map.graph.node_count();
    let directed_roads = map.graph.edge_count();
    let road_km: f32 = map.graph.edge_weights().map(|r| r.length).sum::<f32>() / 2000.0;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let generated = create_random_commutes_with_rng(&map, count, &mut rng);
    let mut config = SimulationConfig::new(seconds as f32, 0.1, map);
    config.minimum_gap = 2.0;
    let mut engine =
        SimulationEngine::new_with_commutes(config, generated.vehicles, generated.commute_plans);
    engine.set_seed(seed);
    let setup_s = setup.elapsed().as_secs_f64();
    let mut times = Vec::new();
    let mut physics = Vec::new();
    let mut peak = 0;
    let mut peak_moving = 0;
    let mut high_load = Vec::new();
    let mut peak_step_ms: f64 = 0.0;
    let mut active_sum = 0usize;
    let mut max_bytes = 0;
    let wall = Instant::now();
    for tick in 0..seconds * 10 {
        let start = Instant::now();
        engine.step();
        engine.current_time = (tick + 1) as f32 / 10.0;
        let phys_ms = start.elapsed().as_secs_f64() * 1000.0;
        let packet = ServerPacket::VehicleUpdate {
            vehicles: engine
                .vehicles
                .iter()
                .map(|v| serialize_vehicle(v, &engine.config.map))
                .collect(),
            traffic_lights: serialize_traffic_lights(
                &engine.config.map,
                &engine.controller_green_road_ids,
            ),
            simulation_time_s: engine.current_time,
            playback_speed: 1,
            snapshot_interval_s: 0.1,
        };
        let bytes = serde_json::to_vec(&packet).unwrap();
        std::hint::black_box(&bytes);
        max_bytes = max_bytes.max(bytes.len());
        // Include buffer and JSON object destruction in the per-step budget.
        drop(bytes);
        drop(packet);
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        times.push(ms);
        physics.push(phys_ms);
        let active = engine
            .vehicles
            .iter()
            .filter(|v| v.state == VehicleState::OnRoad)
            .count();
        let moving = engine
            .vehicles
            .iter()
            .filter(|v| v.state == VehicleState::OnRoad && v.velocity > 0.1)
            .count();
        if active > peak {
            peak = active;
            peak_step_ms = ms;
        }
        peak_moving = peak_moving.max(moving);
        active_sum += active;
        if active >= count * 8 / 10 {
            high_load.push(ms);
        }
    }
    let elapsed = wall.elapsed().as_secs_f64();
    let arrived = engine
        .vehicles
        .iter()
        .filter(|v| v.state == VehicleState::Arrived)
        .count();
    let mean = times.iter().sum::<f64>() / times.len() as f64;
    let missed = times.iter().filter(|&&t| t > 100.0).count();
    times.sort_by(f64::total_cmp);
    physics.sort_by(f64::total_cmp);
    high_load.sort_by(f64::total_cmp);
    println!(
        "{}",
        serde_json::json!({
            "high_load_threshold": count*8/10,
            "high_load_steps": high_load.len(),
            "high_load_p99_ms": if high_load.is_empty(){None}else{Some(percentile(&high_load,0.99))},
            "grid": n,
            "commutes": count,
            "seed": seed,
            "sim_seconds": seconds,
            "vehicles_total": engine.vehicles.len(),
            "nodes": nodes,
            "directed_roads": directed_roads,
            "road_km": road_km,
            "side_km": (n-1) as f64*0.15,
            "area_km2": ((n-1) as f64*0.15).powi(2),
            "setup_s": setup_s,
            "wall_s": elapsed,
            "peak_on_road": peak,
            "peak_moving": peak_moving,
            "mean_on_road": active_sum as f64/times.len() as f64,
            "peak_step_ms": peak_step_ms,
            "mean_ms": mean,
            "p95_ms": percentile(&times,0.95),
            "p99_ms": percentile(&times,0.99),
            "max_ms": times.last(),
            "physics_p99_ms": percentile(&physics,0.99),
            "over_100ms": missed,
            "steps": times.len(),
            "max_packet_bytes": max_bytes,
            "arrived": arrived
        })
    );
}
