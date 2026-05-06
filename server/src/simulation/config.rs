use crate::map::model::{Map, MapSettings};

#[derive(Clone)]
pub struct ScoreWeights {
    pub time: f32,
    pub success: f32,
    pub pollution: f32,
    pub infrastructure: f32,
}

impl ScoreWeights {
    pub fn from_settings(s: &MapSettings) -> Self {
        Self {
            time: s.time_weight,
            success: s.success_weight,
            pollution: s.pollution_weight,
            infrastructure: s.infrastructure_weight,
        }
    }
}

#[derive(Clone)]
pub struct SimulationConfig {
    pub start_time: f32,
    pub end_time: f32,
    pub time_step: f32,
    pub minimum_gap: f32, // between vehicles
    pub map: Map,
    pub score_weights: ScoreWeights,
}

impl SimulationConfig {
    pub fn new(end_time: f32, time_step: f32, map: Map) -> Self {
        let score_weights = ScoreWeights::from_settings(&map.settings);
        Self {
            start_time: 0.0,
            end_time,
            time_step,
            minimum_gap: 1.0,
            score_weights,
            map,
        }
    }
}

pub const MAX_SPEED: f32 = 42.0;
pub const ACCELERATION_EXPONENT: f32 = 4.0;

pub const LOOK_AHEAD: f32 = 0.1;
pub const STOP_DWELL_TIME: f32 = 1.0;
pub const IMPATIENCE_RATE: f32 = 0.05;
pub const MIN_CREEP_SPEED: f32 = 1.0;
pub const LANE_WIDTH: f32 = 7.5;
