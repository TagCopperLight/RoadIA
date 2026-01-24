use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub start_time_s: f32,
    pub end_time_s: f32,
    pub time_step_s: f32,
}

pub const MAX_SPEED_KMH: f32 = 150.0;
