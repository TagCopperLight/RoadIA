use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Road {
    pub id: u32,
    pub road_type: RoadType,
    pub lanes: u32,
    pub max_speed_kmh: f32,
    pub length_m: f32,
    pub is_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RoadType {
    Bilateral,
    Unilateral,
}
