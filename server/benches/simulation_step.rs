use criterion::{black_box, criterion_group, criterion_main, BatchSize, Criterion};
use petgraph::Direction;

use server::map::editor::add_traffic_light_controller;
use server::map::intersection::{build_intersections, IntersectionKind};
use server::map::model::Map;
use server::simulation::config::SimulationConfig;
use server::simulation::engine::{Simulation, SimulationEngine};
use server::simulation::vehicle::{TripRequest, Vehicle, VehicleKind, VehicleSpec, VehicleType};

const STEP_COUNT: usize = 100;

fn build_engine(node_count: usize, vehicle_count: usize) -> SimulationEngine {
    let mut map = Map::new();
    let mut node_ids = Vec::with_capacity(node_count);

    for index in 0..node_count {
        node_ids.push(map.add_intersection(
            IntersectionKind::Intersection,
            index as f32 * 100.0,
            0.0,
        ));
    }

    for pair in node_ids.windows(2) {
        map.add_two_way_road(pair[0], pair[1], 1, 25.0, 100.0);
    }

    build_intersections(&mut map);

    if node_ids.len() > 2 {
        let controller_node_id = node_ids[node_ids.len() / 2];
        if let Some(controller_idx) = map.find_node(controller_node_id) {
            let mut phase_link_ids = Vec::new();
            for edge in map.graph.edges_directed(controller_idx, Direction::Incoming) {
                for lane in &edge.weight().lanes {
                    for link in &lane.links {
                        phase_link_ids.push(link.id);
                    }
                }
            }

            if !phase_link_ids.is_empty() {
                let _ = add_traffic_light_controller(
                    &mut map,
                    controller_node_id,
                    vec![(phase_link_ids, 10_000.0, 0.0)],
                )
                .expect("benchmark traffic light controller");
            }
        }
    }

    let origin = map.find_node(node_ids[0]).expect("origin node");
    let destination = map
        .find_node(*node_ids.last().expect("destination node"))
        .expect("destination node");

    let spec = VehicleSpec::new(VehicleKind::Car, 25.0, 2.0, 2.5, 1.0, 4.5);
    let mut vehicles = Vec::with_capacity(vehicle_count);

    for index in 0..vehicle_count {
        let mut vehicle = Vehicle::new(
            index as u64,
            spec,
            TripRequest {
                origin,
                destination,
                departure_time: index as f32 * 0.05,
            },
            VehicleType::Essence,
        );

        assert!(vehicle.update_path(&map), "benchmark route should exist");
        vehicles.push(vehicle);
    }

    let config = SimulationConfig::new(node_count as f32 * 40.0, 0.1, map);
    SimulationEngine::new(config, vehicles)
}

fn run_steps(engine: &mut SimulationEngine, step_count: usize) {
    for _ in 0..step_count {
        engine.step();
        engine.current_time += engine.config.time_step;
    }
}

fn bench_step_throughput(c: &mut Criterion) {
    let small = build_engine(6, 32);
    let large = build_engine(18, 256);

    c.bench_function("simulation_step/small", |b| {
        b.iter_batched(
            || small.clone(),
            |mut engine| {
                run_steps(black_box(&mut engine), STEP_COUNT);
            },
            BatchSize::SmallInput,
        );
    });

    c.bench_function("simulation_step/large", |b| {
        b.iter_batched(
            || large.clone(),
            |mut engine| {
                run_steps(black_box(&mut engine), STEP_COUNT);
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(simulation_step_benches, bench_step_throughput);
criterion_main!(simulation_step_benches);
