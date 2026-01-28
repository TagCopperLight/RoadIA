use crate::simulation::engine::{Simulation, SimulationEngine};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{map::model::Map, simulation::config::SimulationConfig};
    // use crate::simulation::config::SimulationConfig; // Engine creates config now
    // use crate::simulation::vehicle::Vehicle;

    #[test]
    fn test_simulation_engine_creation_and_step() {
        let map = Map::default();
        let vehicles = vec![];
        let config = SimulationConfig {
            start_time_s: 0.0,
            end_time_s: 10.0,
            time_step_s: 1.0,
            acceleration_exponent: 4.0,
            minimum_gap: 1.0,
            map,
        };
        let mut sim = SimulationEngine::new(config, vehicles);

        assert_eq!(sim.current_time, 0.0);
        assert_eq!(sim.config.end_time_s, 10.0);

        sim.step();
        // step() does NOT increment current_time, run() does.
        // check if step runs without panic

        sim.run();
        assert!(sim.current_time >= 10.0);
    }
}
