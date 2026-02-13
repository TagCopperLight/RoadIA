use crate::simulation::config::MAX_SPEED;

#[derive(Clone)]
pub struct Road {
    pub id: u32,
    pub lane_count: u8,
    pub speed_limit: f32,
    pub length: f32,
    pub is_blocked: bool,
    pub can_overtake: bool,
}

impl Road {
    pub fn new(
        id: u32,
        lane_count: u8,
        speed_limit: f32,
        length: f32,
        is_blocked: bool,
        can_overtake: bool,
    ) -> Self {
        Self {
            id,
            lane_count,
            speed_limit: speed_limit.clamp(1.0, MAX_SPEED),
            length,
            is_blocked,
            can_overtake,
        }
    }
}
