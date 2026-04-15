<!-- Auto documentation: extracted from server/src/simulation/mod.rs -->

# `simulation` (module)

Regroupe les sous-modules `config`, `engine`, `kinematics`, `vehicle`.

## Structure

- `config` — constantes et `SimulationConfig`.
- `engine` — `SimulationEngine` implémentant `Simulation` trait.
- `kinematics` — fonctions physiques utilitaires (`arrival_time`, `v_stop_at`, ...).
- `vehicle` — types `Vehicle`, `VehicleSpec`, `DrivePlanEntry` et fonctions associées.

## Intégration

- L'engine orchestre les appels aux sous-modules et expose l'API de simulation via `SimulationInstance`.
