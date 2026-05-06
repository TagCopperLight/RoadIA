#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommutePlanState {
    OutboundPending,
    OutboundRunning,
    WaitingForReturnDeparture,
    ReturnRunning,
    Completed,
    Failed,
}

#[derive(Clone, Debug)]
pub struct CommutePlan {
    pub id: u64,
    pub outbound_vehicle_id: u64,
    pub return_vehicle_id: u64,
    pub waiting_time_s: f32,
    pub state: CommutePlanState,
}

impl CommutePlan {
    pub fn new(id: u64, outbound_vehicle_id: u64, return_vehicle_id: u64, waiting_time_s: f32) -> Self {
        Self {
            id,
            outbound_vehicle_id,
            return_vehicle_id,
            waiting_time_s: waiting_time_s.max(0.0),
            state: CommutePlanState::OutboundPending,
        }
    }
}