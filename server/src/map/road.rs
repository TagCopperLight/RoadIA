use crate::simulation::config::MAX_SPEED_MS;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Road {
    pub id: u32,
    pub lane_count: u8,
    pub speed_limit_ms: u8,
    pub length_m: f32,
    pub is_blocked: bool,
    pub can_overtake: bool,
}

impl Road {
    pub fn new(
        id: u32,
        lane_count: u8,
        speed_limit_ms: u8,
        length_m: f32,
        is_blocked: bool,
        can_overtake: bool,
    ) -> Self {
        Self {
            id,
            lane_count,
            speed_limit_ms: speed_limit_ms.max(1).min(MAX_SPEED_MS),
            length_m,
            is_blocked,
            can_overtake,
        }
    }
}
