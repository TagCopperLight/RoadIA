use tokio::sync::broadcast;

use crate::api::websocket::{run_score_request_with_progress, ServerPacket};
use crate::simulation::engine::{Simulation, SimulationEngine};
use crate::test::{make_minimal_straight_map, make_sim_config};

fn collect_packets(rx: &mut broadcast::Receiver<ServerPacket>) -> Vec<ServerPacket> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

#[test]
fn score_progress_emits_multiple_updates_before_final_score() {
    let map = make_minimal_straight_map();
    let engine = SimulationEngine::new(make_sim_config(map, 25.0), vec![]);
    let (tx, mut rx) = broadcast::channel(256);

    run_score_request_with_progress(engine, tx);

    let packets = collect_packets(&mut rx);
    let progress_values: Vec<f32> = packets
        .iter()
        .filter_map(|packet| match packet {
            ServerPacket::ScoreProgress { progress } => Some(*progress),
            _ => None,
        })
        .collect();

    assert!(progress_values.len() > 1, "expected multiple progress packets");
    assert!(progress_values.iter().any(|progress| *progress < 100.0));
    assert_eq!(*progress_values.last().unwrap(), 100.0);
    assert!(matches!(packets.last(), Some(ServerPacket::Score { .. })));
}

#[test]
fn score_progress_short_run_still_finishes_at_hundred() {
    let map = make_minimal_straight_map();
    let engine = SimulationEngine::new(make_sim_config(map, 0.1), vec![]);
    let (tx, mut rx) = broadcast::channel(16);

    run_score_request_with_progress(engine, tx);

    let packets = collect_packets(&mut rx);
    let progress_values: Vec<f32> = packets
        .iter()
        .filter_map(|packet| match packet {
            ServerPacket::ScoreProgress { progress } => Some(*progress),
            _ => None,
        })
        .collect();

    assert!(progress_values.len() >= 2, "expected at least one intermediate update plus completion");
    assert!(progress_values.first().copied().unwrap() < 100.0);
    assert_eq!(*progress_values.last().unwrap(), 100.0);
    assert!(matches!(packets.last(), Some(ServerPacket::Score { .. })));
}