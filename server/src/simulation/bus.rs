use petgraph::graph::NodeIndex;
use crate::simulation::vehicle::VehicleSpec;

/// Bus specifications with realistic physics parameters
pub struct BusSpecifications;

impl BusSpecifications {
    pub const MAX_SPEED: f32 = 25.0;           // 90 km/h max
    pub const MAX_ACCELERATION: f32 = 1.5;    // 1.5 m/s² acceleration
    pub const COMFORTABLE_DECELERATION: f32 = 2.0; // 2 m/s² deceleration
    pub const REACTION_TIME: f32 = 1.5;       // 1.5s reaction time
    pub const LENGTH: f32 = 12.0;             // 12m long

    pub fn default_spec() -> VehicleSpec {
        VehicleSpec::new(
            crate::simulation::vehicle::VehicleKind::Bus,
            Self::MAX_SPEED,
            Self::MAX_ACCELERATION,
            Self::COMFORTABLE_DECELERATION,
            Self::REACTION_TIME,
            Self::LENGTH,
        )
    }
}

/// Simple bus info for tracking
#[derive(Clone, Copy, Debug)]
pub struct BusInfo {
    pub bus_id: u64,
}

impl BusInfo {
    pub fn new(bus_id: u64) -> Self {
        Self { bus_id }
    }
}

/// Minimal bus state for tracking stop times
#[derive(Clone, Debug)]
pub struct BusVehicleState {
    pub bus_id: u64,
    pub stop_time_remaining: f32,
}

impl BusVehicleState {
    pub fn new(bus_id: u64) -> Self {
        Self {
            bus_id,
            stop_time_remaining: 0.0,
        }
    }

    pub fn update_stop_time(&mut self, delta_time: f32) {
        if self.stop_time_remaining > 0.0 {
            self.stop_time_remaining -= delta_time;
            if self.stop_time_remaining <= 0.0 {
                self.stop_time_remaining = 0.0;
            }
        }
    }

    pub fn is_stopped(&self) -> bool {
        self.stop_time_remaining > 0.0
    }
}

/// Simple bus route definition
#[derive(Clone, Debug)]
pub struct BusStop {
    pub intersection: NodeIndex,
    pub name: String,
}

impl BusStop {
    pub fn new(intersection: NodeIndex, name: String) -> Self {
        Self { intersection, name }
    }
}

/// Bus route with stops
#[derive(Clone, Debug)]
pub struct BusRoute {
    pub id: u32,
    pub stops: Vec<BusStop>,
    pub name: String,
}

impl BusRoute {
    pub fn new(id: u32, stops: Vec<BusStop>, name: String) -> Self {
        Self { id, stops, name }
    }

    pub fn stop_count(&self) -> usize {
        self.stops.len()
    }

    pub fn get_stop(&self, index: usize) -> Option<&BusStop> {
        self.stops.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_specs() {
        let spec = BusSpecifications::default_spec();
        assert_eq!(spec.max_speed, BusSpecifications::MAX_SPEED);
        assert_eq!(spec.length, BusSpecifications::LENGTH);
    }

    #[test]
    fn test_bus_state() {
        let mut state = BusVehicleState::new(1);
        assert!(!state.is_stopped());
        
        state.stop_time_remaining = 5.0;
        assert!(state.is_stopped());
        
        state.update_stop_time(3.0);
        assert_eq!(state.stop_time_remaining, 2.0);
        
        state.update_stop_time(3.0);
        assert_eq!(state.stop_time_remaining, 0.0);
        assert!(!state.is_stopped());
    }

    #[test]
    fn test_bus_route() {
        let stops = vec![
            BusStop::new(NodeIndex::new(0), "Stop A".to_string()),
            BusStop::new(NodeIndex::new(1), "Stop B".to_string()),
        ];
        let route = BusRoute::new(1, stops, "Route 1".to_string());
        assert_eq!(route.stop_count(), 2);
    }
}


