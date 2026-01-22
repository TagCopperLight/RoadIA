use serde_json::Value;
use std::sync::{Arc, Mutex};

use crate::simulation::vehicle::Vehicle;

#[derive(Clone)]
pub struct Handle {
    pub vehicles: Arc<Mutex<Vec<Vehicle>>>,
    pub map: Arc<Mutex<Value>>,
}

impl Handle {
    pub fn new() -> Self {
        Self {
            vehicles: Arc::new(Mutex::new(Vec::new())),
            map: Arc::new(Mutex::new(serde_json::json!({}))),
        }
    }

    pub fn update_vehicles(&self, vehicles: Vec<Vehicle>) {
        let mut lock = self.vehicles.lock().unwrap();
        *lock = vehicles;
    }

    pub fn snapshot_vehicles(&self) -> Vec<Vehicle> {
        self.vehicles.lock().unwrap().clone()
    }

    pub fn set_map(&self, map: Value) {
        let mut lock = self.map.lock().unwrap();
        *lock = map;
    }

    pub fn snapshot_map(&self) -> Value {
        self.map.lock().unwrap().clone()
    }
}
