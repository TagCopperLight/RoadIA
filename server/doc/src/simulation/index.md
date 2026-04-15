# Simulation overview

Le sous-système `simulation` contient la configuration, l'engine, la cinématique et le modèle `Vehicle`.

Modules:
- `config` : constantes et structure `SimulationConfig`.
- `engine` : `SimulationEngine` implémentant la logique de pas temporel (`step`) et orchestration des véhicules.
- `kinematics` : calculs de vitesse, temps d'arrivée et fonctions physiques.
- `vehicle` : modèle `Vehicle`, `VehicleSpec`, état et fonctions utilitaires (par ex. `fastest_path`).
