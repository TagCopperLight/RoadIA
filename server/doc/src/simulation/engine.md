# Engine details

Fichier: `src/simulation/engine.rs`.

Responsabilités:
- `SimulationEngine` contient l'état: `vehicles`, `vehicles_by_lane`, `link_states`, `green_links`, `current_time`.
- Cycle principal: `handle_departures()`, `plan_movements()`, `register_approaches()`, `advance_traffic_lights()`, `execute_movements()`, `flush_transfers()`.
- Génération des `DrivePlanEntry` pour chaque véhicule et mécanismes d'enregistrement d'approches pour résoudre conflits.
- `get_score()` délègue au module `scoring`.

Points d'attention:
- Le moteur met à jour `vehicles` chaque pas de temps (`time_step`).
- Gestion de l'état des feux via `traffic_light_states` et génération de `green_links`.

Recommandations pour contributeurs:
- Tests unitaires couvrant `rebuild_drive_plan`, `is_link_open`, et transitions de feux.
