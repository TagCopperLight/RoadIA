use crate::simulation::vehicle::Vehicle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SimulationMetrics {
    pub travel_times_s: Vec<u32>,
    pub total_fuel_l: f32,
    pub total_co2_g: f32,
}

impl SimulationMetrics {
    pub fn collect_from_vehicle(&mut self, vehicle: &Vehicle) {
        if let Some(arrival) = vehicle.trip.return_time_s {
            let travel: u32 = arrival.saturating_sub(vehicle.trip.departure_time_s);
            self.travel_times_s.push(travel);
            self.total_fuel_l += vehicle.fuel_used_l;
            self.total_co2_g += vehicle.co2_emitted_g;
        }
    }

    pub fn avg_travel_time_s(&self) -> Option<f32> {
        if self.travel_times_s.is_empty() {
            return None;
        }
        let sum: u32 = self.travel_times_s.iter().copied().sum();
        Some(sum as f32 / self.travel_times_s.len() as f32)
    }

    pub fn min_travel_time_s(&self) -> Option<u32> {
        self.travel_times_s.iter().copied().min()
    }

    pub fn max_travel_time_s(&self) -> Option<u32> {
        self.travel_times_s.iter().copied().max()
    }
}
