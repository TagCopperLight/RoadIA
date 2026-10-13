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

/// Safety margin (s) kept between the time windows of two conflicting vehicles.
pub const CONFLICT_TIME_GAP: f32 = 0.5;
/// Time (s) a vehicle must stand still at a stop line before it may proceed.
pub const STOP_DWELL_TIME: f32 = 1.0;
/// Impatience gained per second spent waiting at a stop line (reaches 1.0 after 20 s).
pub const IMPATIENCE_RATE: f32 = 0.05;
/// Lowest speed a vehicle plans to cross a junction at.
pub const MIN_CREEP_SPEED: f32 = 1.0;
/// Below this speed (m/s) a vehicle counts as stopped.
pub const STOPPED_SPEED: f32 = 0.5;
/// Distance (m), on top of the minimum gap, within which a stopped vehicle is "at" its stop line.
pub const STOP_LINE_TOLERANCE: f32 = 3.0;
/// Minimum time (s) between two vehicles of the same queue crossing the stop line.
pub const QUEUE_HEADWAY: f32 = 1.0;
/// How far ahead (s) the free space behind a moving vehicle is extrapolated when
/// deciding whether a junction exit has room.
pub const EXIT_SPACE_LOOKAHEAD: f32 = 1.5;
/// Deceleration (as a multiple of the comfortable one) a vehicle is willing to use to
/// stop for a red light it is already very close to.
pub const EMERGENCY_DECEL_FACTOR: f32 = 3.0;
pub const LANE_WIDTH: f32 = 7.5;

pub const RECKLESS_PROBABILITY: f32 = 0.01;
pub const NAVIGATION_MISTAKE_PROBABILITY: f32 = 0.0005;

pub const LC2013_COOLDOWN: f32 = 3.0;
pub const LC2013_SPEED_GAIN_THRESHOLD: f32 = 1.5;
pub const LC2013_SAFE_GAP_FRONT: f32 = 10.0;
pub const LC2013_SAFE_GAP_REAR: f32 = 6.0;
pub const LC2013_MAX_DECEL_FOLLOWER: f32 = 3.0;