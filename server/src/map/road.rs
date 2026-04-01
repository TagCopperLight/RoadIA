use crate::simulation::config::{LANE_WIDTH, MAX_SPEED};

#[derive(Clone, Debug, PartialEq)]
pub enum LinkType {
    Yield,
    Priority,
    Stop,
    TrafficLight,
}

#[derive(Clone)]
pub struct Road {
    pub id: u32,
    pub length: f32,
    pub speed_limit: f32,
    pub lane_width: f32,

    pub lanes: Vec<Lane>,
}

#[derive(Clone)]
pub struct Lane {
    pub id: u32,
    pub road_id: u32,
    pub length: f32,
    pub speed_limit: f32,

    pub links: Vec<Link>,
}

#[derive(Clone, Debug)]
pub struct FoeLink {
    pub id: u32,
    pub link_type: LinkType,
    pub entry: (f32, f32),
}

#[derive(Clone)]
pub struct Link {
    pub id: u32,
    pub lane_origin_id: u32,
    pub lane_destination_id: u32,
    pub via_internal_lane_id: u32,
    pub destination_road_id: u32,
    pub link_type: LinkType,
    pub entry: (f32, f32),
    pub junction_center: (f32, f32),
    pub foe_links: Vec<FoeLink>,
    pub foe_internal_lane_ids: Vec<u32>,
}

impl Road {
    pub fn new(
        id: u32,
        lane_count: u8,
        speed_limit: f32,
        length: f32,
    ) -> Self {
        let mut lanes = Vec::new();
        for i in 0..lane_count {
            lanes.push(Lane {
                id: i as u32,
                road_id: id,
                length,
                speed_limit: speed_limit.clamp(1.0, MAX_SPEED),
                links: Vec::new(),
            });
        }
        Self {
            id,
            length,
            speed_limit: speed_limit.clamp(1.0, MAX_SPEED),
            lane_width: LANE_WIDTH,
            lanes,
        }
    }
}
