use crate::map::model::Map;

pub struct SimulationConfig {
    pub start_time: f32,
    pub end_time: f32,
    pub time_step: f32,
    pub minimum_gap: f32, //between vehicles
    pub air_density: f32, //en Kg/m3
    pub gravity_coefficient: f32, //en m/s²
    //Les paramètres suivants pondèrent les facteurs du score
    pub time_weight : f32,
    pub success_weight: f32,
    pub pollution_weight: f32,
    pub infrastructure_weight: f32,
    pub map: Map,
}

impl SimulationConfig {
    pub fn new(end_time: f32, time_step: f32, map: Map) -> Self {
        Self {
            start_time: 0.0,
            end_time,
            time_step,
            minimum_gap: 1.0,
            air_density: 1.225,
            gravity_coefficient: 9.81,
            time_weight: 0.4,
            success_weight: 0.2,
            pollution_weight: 0.2,
            infrastructure_weight: 0.2,
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