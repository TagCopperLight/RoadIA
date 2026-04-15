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

API et descriptions fonctionnelles (paramètres & retours)

`SimulationEngine::new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> SimulationEngine`:
- `config`: configuration de simulation (map, time_step, start/end times, minimum_gap, etc.).
- `vehicles`: liste initiale de `Vehicle` (doit contenir `trip`/`origin`/`destination`).

Cycle interne (**step**) — fonctions principales:
- `handle_departures(&mut self)` — active les véhicules dont `departure_time <= current_time` si espace suffisant sur la voie d'origine; met `state = OnRoad` et insère dans `vehicles_by_lane`.
- `plan_movements(&mut self)` — pour chaque véhicule en `OnRoad` et non engagé dans un `Internal` lane, appelle `rebuild_drive_plan(vidx)` pour générer `DrivePlanEntry` sur l'itinéraire anticipé.
- `rebuild_drive_plan(&mut self, vidx: usize)` — construit la séquence de `DrivePlanEntry` pour le véhicule `vidx`:
	- calcule `braking_horizon` en fonction de `v^2/(2*d_max)` et d'un buffer;
	- itère sur le `path` et pour chaque jonction identifie `in_edge`/`out_edge`, calcule `arrival_time` via `kinematics::arrival_time(...)`, `leave_time` via `kinematics::leave_time(...)`, et remplit `DrivePlanEntry` (voir `simulation::vehicle::DrivePlanEntry` pour champs).
- `register_approaches(&mut self)` — lit les `DrivePlanEntry` des véhicules et inscrit `ApproachData` dans `link_states` (clé: `link_id`) afin que les arbitres (is_link_open) puissent décider.
- `advance_traffic_lights(&mut self)` — incrémente le temps des phases et met à jour `green_links` en conséquence.
- `execute_movements(&mut self)` — pour chaque véhicule OnRoad appelle `execute_vehicle(vidx, lane_id)`.

Fonctions utilitaires importantes:
- `determine_safe_speed(&self, vidx: usize) -> (f32, Option<f32>)` — renvoie `(speed_to_use, optional_stop_distance)`:
	- si `is_link_open(...)` retourne vrai → renvoie `v_pass` (point of no return logic) sinon renvoie `v_wait` et la distance de freinage demandée (Some(distance)).
- `find_link(&self, link_id: u32) -> Option<map::road::Link>` — renvoie le clone du link depuis `link_directory`.
- `lane_length`, `lane_speed_limit` — exposent propriétés utilitaires pour une lane donnée du véhicule.

Points de documentation à surveiller (actions recommandées):
- Documenter précisément les unités et marges temporelles utilisées par `arrival_time`/`leave_time` (fonctions dans `simulation::kinematics`).
- Ajouter exemples numériques dans `simulation/kinematics.md` démontrant `arrival_time` et `v_stop_at` pour scénarios typiques.
- Documenter la structure de `link_states` et `ApproachData` (déjà couvert dans `map/intersection.md` mais à lier ici).
