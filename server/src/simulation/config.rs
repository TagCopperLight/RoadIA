use crate::map::model::{Map};

pub struct SimulationConfig {
    pub start_time_s: f32,
    pub end_time_s: f32,
    pub time_step_s: f32,
    pub minimum_gap: f32, //between vehicles

    pub map: Map,
}

pub const MAX_SPEED: u8 = 42;
pub const ACCELERATION_EXPONENT: f32 = 4.0;