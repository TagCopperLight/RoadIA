<!-- Auto documentation: extracted from server/src/simulation/config.rs -->

# `simulation::config`

Paramètres et constantes du moteur de simulation.

## Constantes notables

- `time_step` / `SimulationConfig.time_step` — résolution temporelle (s).
- `start_time`, `end_time` — bornes temporelles de la simulation.
- `MIN_CREEP_SPEED`, `IMPATIENCE_RATE`, `LOOK_AHEAD`, `STOP_DWELL_TIME` — constantes comportementales utilisées par l'engine.
- `MAX_SPEED`, `LANE_WIDTH`, `ACCELERATION_EXPONENT` — constantes physiques/paramètres de véhicule.

## `SimulationConfig` struct

- Contient:
  - `start_time`, `end_time`, `time_step`, `minimum_gap`, `map` (Map)

## Usage

- Le `SimulationEngine::new(config, vehicles)` consomme ces paramètres pour initialiser l'état.
- Ajuster `time_step` affecte précision et coûts CPU; `minimum_gap` influence départs et sécurité des insertions.

## Recommandations

- Pour tests reproductibles: documenter les valeurs de constantes et les exposer via configuration si nécessaire.
# `src/simulation/config.rs`

Overview
- Contains `SimulationConfig` which centralizes runtime tuning parameters for the simulation engine, and a set of constants used across the kinematic and planning code.

`SimulationConfig`
- Fields include `start_time`, `end_time`, `time_step`, `minimum_gap` and the `map` instance used for routing and collision checks.
- Use the constructor to create a config with sensible defaults; the config is passed to `SimulationEngine` at initialization.

Constants
- The module defines project-wide constants (e.g. `MAX_SPEED`, `LOOK_AHEAD`, `STOP_DWELL_TIME`, `IMPATIENCE_RATE`, `LANE_WIDTH`) that encode default physical and behavioral settings. Tuning these values changes global simulation behavior.

Notes
- Keep `SimulationConfig` as the canonical place to change simulation resolution and safety margins; tests and experiments should record config values used for reproducibility.
