use rand::Rng;
use rand_distr::{Beta, Distribution};

const COMMUTE_TIME_WINDOW_S: f32 = 43_200.0;
const DEPARTURE_BETA_ALPHA: f64 = 6.33;
const DEPARTURE_BETA_BETA: f64 = 3.67;
const WAIT_BETA_ALPHA: f64 = 7.25;
const WAIT_BETA_BETA: f64 = 2.75;

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
    pub outbound_departure_time_s: f32,
    pub return_waiting_time_s: f32,
    pub state: CommutePlanState,
}

impl CommutePlan {
    pub fn new(
        id: u64,
        outbound_vehicle_id: u64,
        return_vehicle_id: u64,
        outbound_departure_time_s: f32,
        return_waiting_time_s: f32,
    ) -> Self {
        Self {
            id,
            outbound_vehicle_id,
            return_vehicle_id,
            outbound_departure_time_s: outbound_departure_time_s.max(0.0),
            return_waiting_time_s: return_waiting_time_s.max(0.0),
            state: CommutePlanState::OutboundPending,
        }
    }

    pub fn immediate(
        id: u64,
        outbound_vehicle_id: u64,
        return_vehicle_id: u64,
        simulation_start_time_s: f32,
    ) -> Self {
        Self::new(id, outbound_vehicle_id, return_vehicle_id, simulation_start_time_s, 0.0)
    }

    pub fn random<R: Rng + ?Sized>(
        id: u64,
        outbound_vehicle_id: u64,
        return_vehicle_id: u64,
        simulation_start_time_s: f32,
        rng: &mut R,
    ) -> Self {
        let departure_beta = Beta::new(DEPARTURE_BETA_ALPHA, DEPARTURE_BETA_BETA)
            .expect("valid beta parameters for commute departure time");
        let waiting_beta = Beta::new(WAIT_BETA_ALPHA, WAIT_BETA_BETA)
            .expect("valid beta parameters for commute waiting time");

        let outbound_departure_time_s = loop {
            let sampled_departure = COMMUTE_TIME_WINDOW_S * departure_beta.sample(rng) as f32;
            if sampled_departure >= simulation_start_time_s {
                break sampled_departure;
            }
        };
        let return_waiting_time_s = (COMMUTE_TIME_WINDOW_S * waiting_beta.sample(rng) as f32)
            .clamp(0.0, COMMUTE_TIME_WINDOW_S);

        Self::new(
            id,
            outbound_vehicle_id,
            return_vehicle_id,
            outbound_departure_time_s,
            return_waiting_time_s,
        )
    }
}