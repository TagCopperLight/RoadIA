use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Road {
    pub id: u32,
    pub lane_count: u8,
    pub speed_limit_kmh: u8,
    pub length_m: f32,
    pub is_blocked: bool,
    pub can_overtake: bool,
}
