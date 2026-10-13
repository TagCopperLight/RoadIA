//! Engine stress harness: runs a scenario and reports gridlocks, overlaps and step timings.
//!
//! Usage:
//!   cargo run --release --example stress -- <scenario> [commutes] [seed] [sim_seconds]
//!
//! Scenarios:
//!   map:<path.json>   a map saved by the editor
//!   grid:<n>          an n x n grid of two-way single-lane roads
//!   grid2:<n>         an n x n grid of two-way two-lane roads
//!
//! Environment:
//!   RINGS=1    mark short one-way loops as roundabouts (emulates re-importing an old
//!              saved OSM map with roundabout detection)
//!   CHAINS=<n> print the wait-for chain of <n> stuck vehicles

use std::collections::HashMap;
use std::time::{Duration, Instant};

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use server::api::runner::map_generator::create_random_commutes_with_rng;
use server::map::intersection::{build_intersections, IntersectionKind};
use server::map::model::Map;
use server::simulation::config::SimulationConfig;
use server::simulation::engine::{Simulation, SimulationEngine};
use server::simulation::vehicle::{LaneId, VehicleState};

const STUCK_AFTER_S: f32 = 120.0;

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

fn load_scenario(spec: &str) -> Map {
    let (kind, arg) = spec.split_once(':').expect("scenario must be kind:arg");
    match kind {
        "map" => {
            let mut map = Map::load(arg).expect("failed to load map");
            map.settings.use_day_night_cycle = false;
            map
        }
        "grid" => grid_map(arg.parse().expect("grid size"), 1),
        "grid2" => grid_map(arg.parse().expect("grid size"), 2),
        _ => panic!("unknown scenario kind {kind}"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let scenario = args.get(1).map(String::as_str).unwrap_or("grid:6");
    let commutes: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(300);
    let seed: u64 = args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1);
    let sim_seconds: f32 = args.get(4).and_then(|s| s.parse().ok()).unwrap_or(3600.0);

    let mut map = load_scenario(scenario);
    map.settings.use_day_night_cycle = false;
    if std::env::var("RINGS").is_ok() {
        // Emulates re-importing a saved OSM map with roundabout detection.
        let rings = short_one_way_loops(&map, 250.0);
        println!("marked {} roundabout roads", rings.len());
        server::map::roundabout::mark_roundabout_entries(&mut map, &rings);
    }
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let generated = create_random_commutes_with_rng(&map, commutes, &mut rng);
    let mut config = SimulationConfig::new(sim_seconds, 0.1, map);
    config.minimum_gap = 2.0;
    let mut engine =
        SimulationEngine::new_with_commutes(config, generated.vehicles, generated.commute_plans);
    engine.set_seed(seed);

    println!(
        "scenario={scenario} vehicles={} seed={seed} sim={sim_seconds}s",
        engine.vehicles.len()
    );

    let mut last_moved = vec![0.0f32; engine.vehicles.len()];
    let mut step_time = Duration::ZERO;
    let mut steps = 0usize;
    let mut overlap_steps = 0usize;
    let mut worst_overlap = 0.0f32;
    let mut next_report = 0.0f32;

    while engine.current_time < engine.config.end_time {
        let start = Instant::now();
        engine.step();
        step_time += start.elapsed();
        steps += 1;
        engine.current_time += engine.config.time_step;
        let now = engine.current_time;

        let mut overlapped = false;
        for indices in engine.vehicles_by_lane.values() {
            for pair in indices.windows(2) {
                let follower = &engine.vehicles[pair[0]];
                let leader = &engine.vehicles[pair[1]];
                let gap = leader.position_on_lane - leader.spec.length - follower.position_on_lane;
                if gap < -0.5 {
                    overlapped = true;
                    worst_overlap = worst_overlap.min(gap);
                }
            }
        }
        if overlapped {
            overlap_steps += 1;
        }

        for (i, v) in engine.vehicles.iter().enumerate() {
            if v.state != VehicleState::OnRoad || v.velocity > 0.1 {
                last_moved[i] = now;
            }
        }

        if now >= next_report {
            next_report += 300.0;
            let on_road = engine.vehicles.iter().filter(|v| v.state == VehicleState::OnRoad).count();
            let arrived = engine.vehicles.iter().filter(|v| v.state == VehicleState::Arrived).count();
            let stuck = last_moved.iter().filter(|&&t| now - t > STUCK_AFTER_S).count();
            println!(
                "t={now:7.1} on_road={on_road:5} arrived={arrived:5} stuck={stuck:5} avg_step={:?}",
                step_time / steps as u32
            );
        }
        if engine.all_vehicles_arrived {
            break;
        }
    }

    let now = engine.current_time;
    let arrived = engine.vehicles.iter().filter(|v| v.state == VehicleState::Arrived).count();
    let stuck: Vec<usize> = (0..engine.vehicles.len())
        .filter(|&i| now - last_moved[i] > STUCK_AFTER_S)
        .collect();

    println!("--- summary ---");
    println!("end t={now:.1} arrived={arrived}/{}", engine.vehicles.len());
    println!("avg step time: {:?} over {steps} steps", step_time / steps.max(1) as u32);
    println!("steps with overlap: {overlap_steps} (worst gap {worst_overlap:.2} m)");
    println!("stuck vehicles (> {STUCK_AFTER_S}s without moving): {}", stuck.len());

    let mut by_junction: HashMap<String, usize> = HashMap::new();
    for &i in &stuck {
        let v = &engine.vehicles[i];
        let key = match v.current_lane {
            Some(LaneId::Normal(edge, lane)) => {
                let road = &engine.config.map.graph[edge];
                let head = engine
                    .vehicles_by_lane
                    .get(&v.current_lane.unwrap())
                    .and_then(|l| l.last())
                    .copied()
                    == Some(i);
                format!(
                    "road {} lane {lane} -> node {} {} pos={:.1}/{:.1} {:?}",
                    road.id,
                    engine.config.map.graph[engine.config.map.graph.edge_endpoints(edge).unwrap().1].id,
                    if head { "(head of queue)" } else { "" },
                    v.position_on_lane,
                    road.length,
                    engine.blocking_reason(i),
                )
            }
            Some(LaneId::Internal(j, il)) => format!(
                "inside junction {j} lane {il} veh {} pos={:.1} {:?}",
                v.id,
                v.position_on_lane,
                engine.blocking_reason(i)
            ),
            None => "no lane".to_string(),
        };
        *by_junction.entry(key).or_default() += 1;
    }
    let mut keys: Vec<_> = by_junction.into_iter().filter(|(k, _)| k.contains("head") || k.contains("inside")).collect();
    println!("heads/inside: {}", keys.len());
    keys.sort();
    for (k, n) in keys.iter().take(40) {
        println!("  {n:4} x {k}");
    }
    let chains: usize = std::env::var("CHAINS").ok().and_then(|s| s.parse().ok()).unwrap_or(1);
    let heads: Vec<usize> = stuck.iter().copied().filter(|&i| {
        let lane = engine.vehicles[i].current_lane.unwrap();
        engine.vehicles_by_lane.get(&lane).and_then(|l| l.last()) == Some(&i)
    }).collect();
    for &start in heads.iter().step_by((heads.len() / chains.max(1)).max(1)).take(chains) {
        println!("--- wait-for chain from vehicle {} ---", engine.vehicles[start].id);
        print_wait_chain(&engine, start);
    }

    let score = engine.get_score();
    println!("score={:.3} success_rate={:.3}", score.score, score.success_rate);
}

/// Follows "who is this vehicle waiting for" until a vehicle repeats.
fn print_wait_chain(engine: &SimulationEngine, start: usize) {
    use server::simulation::engine::Blocker;
    let index_of = |id: u64| engine.vehicles.iter().position(|v| v.id == id);
    let head_of = |lane: LaneId| engine.vehicles_by_lane.get(&lane).and_then(|l| l.last()).copied();
    let mut seen = Vec::new();
    let mut current = start;
    for _ in 0..60 {
        if seen.contains(&current) {
            println!("  -> cycle back to vehicle {}", engine.vehicles[current].id);
            return;
        }
        seen.push(current);
        let v = &engine.vehicles[current];
        let lane = v.current_lane.unwrap();
        // Not the head: waiting for the vehicle ahead.
        let head = head_of(lane);
        if head != Some(current) {
            let list = &engine.vehicles_by_lane[&lane];
            let slot = list.iter().position(|&i| i == current).unwrap();
            let link_type = v.drive_plan.first().and_then(|e| engine.config.map.graph.edge_weights().flat_map(|r| r.lanes.iter().flat_map(|l| &l.links)).find(|l| l.id == e.link_id).map(|l| l.link_type.clone()));
            println!("  veh {} queued on {:?} behind veh {} (own waited={:.0}, link {:?} {:?})", v.id, lane, engine.vehicles[list[slot + 1]].id, v.waiting_time, v.drive_plan.first().map(|e| e.link_id), link_type);
            current = list[slot + 1];
            continue;
        }
        let reason = engine.blocking_reason(current);
        let entry = v.drive_plan.first();
        let out_lane = entry.and_then(|e| {
            engine.config.map.graph.edge_indices().find_map(|edge| {
                engine.config.map.graph[edge].lanes.iter().flat_map(|l| &l.links).find(|l| l.id == e.link_id).map(|l| {
                    let out = engine.config.map.find_edge(l.destination_road_id).unwrap();
                    LaneId::Normal(out, l.lane_destination_id)
                })
            })
        });
        let link_type = entry.and_then(|e| engine.config.map.graph.edge_weights().flat_map(|r| r.lanes.iter().flat_map(|l| &l.links)).find(|l| l.id == e.link_id).map(|l| (l.id, l.link_type.clone())));
        println!(
            "  veh {} head of {:?} pos={:.1} v={:.2} waited={:.0}s link={:?} blocked by {:?} (exit {:?}, len {:.1})",
            v.id, lane, v.position_on_lane, v.velocity, v.waiting_time, link_type, reason, out_lane,
            out_lane.map(|l| match l { LaneId::Normal(e, _) => engine.config.map.graph[e].length, _ => 0.0 }).unwrap_or(0.0)
        );
        current = match (reason, out_lane) {
            (Some(Blocker::Foe { vehicle_id }) | Some(Blocker::JunctionBusy { vehicle_id }), _) => index_of(vehicle_id).unwrap(),
            (Some(Blocker::ExitFull), Some(out)) => match engine.vehicles_by_lane.get(&out).and_then(|l| l.first()) {
                Some(&rear) => rear,
                None => {
                    // Claimed by vehicles inside the junction.
                    let inside = engine.vehicles.iter().position(|o| matches!(o.current_lane, Some(LaneId::Internal(..))) && o.velocity < 0.1 && !seen.contains(&engine.vehicles.iter().position(|x| x.id == o.id).unwrap()));
                    println!("    exit empty, claimed by vehicles inside junction");
                    match inside { Some(i) => i, None => return }
                }
            },
            _ => {
                println!("    (no further blocker: plan={:?})", v.drive_plan.first());
                let g = &engine.config.map.graph;
                let ids: Vec<u32> = v.path.iter().map(|&n| g[n].id).collect();
                println!("    path_index={} path ids={:?} waypoints={} dest={}", v.path_index, ids, v.waypoints.len(), g[v.trip.destination].id);
                if let (Some(&a), Some(&b), Some(&c)) = (v.path.get(v.path_index), v.path.get(v.path_index + 1), v.path.get(v.path_index + 2)) {
                    let e_in = g.find_edge(a, b);
                    let e_out = g.find_edge(b, c);
                    println!("    in={:?} out={:?}", e_in, e_out);
                    if let (Some(e_in), Some(e_out)) = (e_in, e_out) {
                        let out_id = g[e_out].id;
                        for lane in &g[e_in].lanes {
                            println!("    lane {} links to roads {:?} (want {out_id})", lane.id, lane.links.iter().map(|l| l.destination_road_id).collect::<Vec<_>>());
                        }
                        println!("    junction internal lanes: {}", g[b].internal_lanes.len());
                    }
                }
                return;
            }
        };
    }
}

/// Roads on directed loops of one-way roads shorter than `max_length` (roundabouts).
fn short_one_way_loops(map: &Map, max_length: f32) -> std::collections::HashSet<u32> {
    use petgraph::visit::EdgeRef;
    let g = &map.graph;
    let one_way = |e: petgraph::graph::EdgeIndex| {
        let (a, b) = g.edge_endpoints(e).unwrap();
        g.find_edge(b, a).is_none()
    };
    let mut rings = std::collections::HashSet::new();
    for start in g.edge_indices().filter(|&e| one_way(e)) {
        // Depth-first search for a path back to the start edge's source.
        let origin = g.edge_endpoints(start).unwrap().0;
        let mut stack = vec![(g.edge_endpoints(start).unwrap().1, g[start].length, vec![start])];
        while let Some((node, length, path)) = stack.pop() {
            if node == origin {
                rings.extend(path.iter().map(|&e| g[e].id));
                break;
            }
            if path.len() > 12 {
                continue;
            }
            for edge in g.edges(node) {
                let e = edge.id();
                if one_way(e) && !path.contains(&e) && length + g[e].length <= max_length {
                    let mut next = path.clone();
                    next.push(e);
                    stack.push((edge.target(), length + g[e].length, next));
                }
            }
        }
    }
    rings
}
