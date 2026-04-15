/// Waypoint management for vehicles
/// Handles waypoint operations with minimal complexity

use petgraph::graph::NodeIndex;
use crate::simulation::vehicle::Vehicle;

/// Update a vehicle's waypoints and reset its state for the new route
/// 
/// This function:
/// 1. Replaces the vehicle's waypoint list
/// 2. Resets waypoint tracking index
/// 3. Clears the current path (will be recalculated later)
pub fn set_waypoints(vehicle: &mut Vehicle, waypoints: Vec<NodeIndex>) {
    vehicle.waypoints = waypoints;
    vehicle.current_waypoint_index = 0;
    // Path will be recalculated by the caller who has access to the map
}

/// Clear all waypoints from a vehicle
pub fn clear_waypoints(vehicle: &mut Vehicle) {
    vehicle.waypoints.clear();
    vehicle.current_waypoint_index = 0;
}
