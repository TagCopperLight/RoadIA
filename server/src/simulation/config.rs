use crate::map::model::Map;

pub struct SimulationConfig {
    pub start_time: f32,
    pub end_time: f32,
    pub time_step: f32,
    pub minimum_gap: f32, //between vehicles

    pub map: Map,
}

pub const MAX_SPEED: f32 = 42.0;
pub const ACCELERATION_EXPONENT: f32 = 4.0;

pub const LOOK_AHEAD: f32 = 0.1;
pub const STOP_DWELL_TIME: f32 = 1.0;
pub const IMPATIENCE_RATE: f32 = 0.05;
pub const MIN_CREEP_SPEED: f32 = 1.0;
pub const LANE_WIDTH: f32 = 7.5;