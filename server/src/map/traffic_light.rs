#[derive(Clone)]
pub struct SignalPhase {
    pub green_link_ids: Vec<u32>,
    pub green_duration: f32,
    pub yellow_duration: f32,
}

#[derive(Clone)]
pub struct TrafficLightController {
    pub id: u32,
    pub intersection_id: u32,
    pub phases: Vec<SignalPhase>,
}

pub struct TrafficLightControllerHandle {
    pub controller_id: u32,
}
