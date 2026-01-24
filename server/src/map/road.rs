use crate::simulation::config::MAX_SPEED_KMH;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Road {
    pub id: u32,
    pub lane_count: u8,
    speed_limit_kmh: u8,
    pub length_m: f32,
    pub is_blocked: bool,
    pub can_overtake: bool,
}

impl Road {
    pub fn new(
        id: u32,
        lane_count: u8,
        speed_limit_kmh: u8,
        length_m: f32,
        is_blocked: bool,
        can_overtake: bool,
    ) -> Self {
        Self {
            id,
            lane_count,
            speed_limit_kmh: speed_limit_kmh.min(MAX_SPEED_KMH),
            length_m,
            is_blocked,
            can_overtake,
        }
    }

    pub fn speed_limit_kmh(&self) -> u8 {
        self.speed_limit_kmh
    }

    pub fn set_speed_limit_kmh(&mut self, speed_limit_kmh: u8) {
        self.speed_limit_kmh = speed_limit_kmh.min(MAX_SPEED_KMH);
    }
}
