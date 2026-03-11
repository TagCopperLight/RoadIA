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
    pub succes_weight: f32,
    pub pollution_weight: f32,
    pub map: Map,
}

pub const MAX_SPEED: f32 = 42.0;
pub const ACCELERATION_EXPONENT: f32 = 4.0;