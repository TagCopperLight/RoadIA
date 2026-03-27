use crate::simulation::engine::{Simulation, SimulationEngine};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{map::model::Map, simulation::config::SimulationConfig};

    #[test]
    fn test_simulation_engine_creation_and_step() {
        let map = Map::default();
        let vehicles = vec![];
        let config = SimulationConfig {
            start_time: 0.0,
            end_time: 10.0,
            time_step: 1.0,
            minimum_gap: 1.0,
            map,
        };
        let mut sim = SimulationEngine::new(config, vehicles);

        assert_eq!(sim.current_time, 0.0);
        assert_eq!(sim.config.end_time, 10.0);

        sim.step();

        sim.run();
        assert!(sim.current_time == sim.config.end_time);
    }
}
