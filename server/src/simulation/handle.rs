use std::sync::{Arc, Mutex};

use crate::simulation::vehicle::Vehicle;

#[derive(Clone)]
pub struct Handle {
    pub vehicles: Arc<Mutex<Vec<Vehicle>>>,
}

impl Handle {
    pub fn new() -> Self {
        Self {
            vehicles: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn snapshot_vehicles(&self) -> Vec<Vehicle> {
        self.vehicles.lock().unwrap().clone()
    }
}
