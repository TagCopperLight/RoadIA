<!-- Auto documentation: extracted from server/src/simulation/engine.rs -->

# `simulation::engine`

Description du coeur du moteur de simulation: structures, cycle d'exécution
et responsabilités des principales méthodes.

## `SimulationEngine` (champ par champ)

- `config: SimulationConfig` — paramètres (start_time, end_time, time_step, map, minimum_gap...)
- `vehicles: Vec<Vehicle>` — flotte gérée par l'engine
- `current_time: f32` — horloge simulation
- `vehicles_by_lane: HashMap<LaneId, Vec<usize>>` — index des véhicules par voie (triés back→front)
- `link_states: HashMap<u32, LinkState>` — état d'approche sur chaque link
- `all_vehicles_arrived: bool` — indicateur d'achèvement
- `green_links: HashSet<u32>` — liens ouverts par feux actuels
- `pending_transfers: Vec<PendingTransfer>` — transferts en attente (changement de voie/jonction)
- `traffic_light_states: HashMap<u32, TrafficLightRuntimeState>` — runtime pour contrôleurs de feux
- `link_directory: HashMap<u32, Link>` — copie rapide des links par id

## Cycle d'exécution (`step()`)

`step()` exécute ces phases dans l'ordre:

1. `handle_departures()` — met sur la route les véhicules prêts si l'espace l'autorise
2. `plan_movements()` — pour chaque véhicule `OnRoad`, construis/actualise son `drive_plan`
3. `register_approaches()` — publie les intentions d'arrivée (ApproachData) par link
4. `advance_traffic_lights()` — fait évoluer les phases et calcule `green_links`
5. `execute_movements()` — calcule accélérations, met à jour positions/vitesses
6. `flush_transfers()` — applique changements de voie / sorties / entrées
7. scoring updates (CO2, arrived timestamps)

Chaque pas utilise `config.time_step` pour integration temporelle.

## Méthodes majeures (résumé)

- `rebuild_drive_plan(vidx)`
	- Construit une séquence `DrivePlanEntry` en regardant la `braking_horizon`.
	- Utilise `kinematics::arrival_time` & `leave_time` pour estimer temps et vitesses de passage.

- `rebuild_drive_plan(vidx)`
	- Paramètres:
		- `vidx: usize` — index du véhicule dans `self.vehicles`.
	- Effet: met à jour `self.vehicles[vidx].drive_plan` avec une séquence de `DrivePlanEntry`.
	- Comportement: parcourt le chemin restant du véhicule, calcule `t_arrive`, `t_leave`, vitesses `v_pass`/`v_wait` et distances cumulées jusqu'à une limite `braking_horizon`.

- `determine_safe_speed(vidx) -> (f32, Option<f32>)`
	- Paramètres:
		- `vidx: usize` — index du véhicule.
	- Retourne `(speed, stop_distance)` où `speed` est la vitesse cible sûre et `stop_distance` optionnel indique qu'un arrêt est requis à cette distance.
	- Utilisation: utilisé par `execute_vehicle` pour choisir entre `v_pass` et `v_wait`.

- `execute_vehicle(vidx, lane_id)`
	- Paramètres:
		- `vidx: usize` — index du véhicule.
		- `lane_id: LaneId` — voie courante du véhicule.
	- Effet: calcule accélération via `compute_acceleration`, met à jour `velocity`, `position_on_lane`, `distance_traveled`, `waiting_time`, `impatience`.

- `handle_departures()`
	- Aucun paramètre.
	- Effet: passe les véhicules `WaitingToDepart` à `OnRoad` si la voie de départ a suffisamment d'espace.

- `register_approaches()`
	- Aucun paramètre.
	- Effet: pour chaque `DrivePlanEntry` marqué `set_request`, enregistre une entrée `ApproachData` dans `self.link_states` sous la clé `link_id`.

- `advance_traffic_lights()`
	- Aucun paramètre.
	- Effet: met à jour `traffic_light_states` en incrémentant `time_in_phase`, change de `phase_index` si nécessaire et remplit `green_links` selon la phase active.

- `execute_movements()`
	- Aucun paramètre.
	- Effet: itère véhicules par voie, appelle `execute_vehicle` et gère la progression sur les voies.

- `flush_transfers()`
	- Aucun paramètre.
	- Effet: exécute les transferts accumulés dans `pending_transfers` — suppression de la file des anciennes voies et insertion triée dans les nouvelles.

- `determine_safe_speed(vidx)`
	- Inspecte le premier `DrivePlanEntry` et le `link` associé pour déterminer
		la vitesse sûre (`v_pass`) ou si un arrêt est nécessaire (renvoie `Some(distance)`).

- `execute_vehicle(vidx, lane_id)`
	- Calcule `safe_speed`, inspecte véhicule devant et calcule accélération via `compute_acceleration`.
	- Met à jour `velocity`, `position_on_lane`, `distance_traveled`, `waiting_time` et `impatience`.

- `process_lane_advances`, `enter_junction_or_arrive`, `exit_internal_lane` — gèrent la traversée des jonctions,
	sorties/arrivées et la mise en file des véhicules sur la voie suivante.

## Concurrence et intégration

- `SimulationEngine` est conçu pour être protégé par `Arc<Mutex<..>>` lorsqu'il est utilisé
	par `SimulationInstance` (dans le runtime HTTP/WS).

## Limitations & notes

- La version `run()` est présente mais non utilisée (boucle synchrone); le code préfère
	le modèle asynchrone où `SimulationInstance` spawne une tâche qui appelle `engine.step()`.
- Beaucoup de valeurs seuils (jitter, limites, nombre d'itérations) sont codées en dur et
	exposées via `simulation::config` constants.

---

Si tu veux, je peux générer un diagramme d'activité Mermaid montrant l'ordre des phases `step()`.

# `src/simulation/engine.rs`

Overview
- Central simulation engine that maintains runtime vehicle state, executes the per-step pipeline, and constructs drive plans used for conflict resolution at intersections.

Responsibilities
- Initialize runtime state from `SimulationConfig` and vehicle list, maintain `vehicles_by_lane` ordering, track traffic light runtime state, and publish simulation outputs.

Step pipeline
- A single `step()` advances the simulation by one time step and performs (conceptually): copy previous velocities, dispatch waiting vehicles, rebuild drive plans for on-road vehicles, register link approaches, advance traffic lights, execute vehicle movements (compute safe speeds and positions), flush pending lane transfers, and update scoring/metrics.

Internal helpers
- `rebuild_drive_plan`: lookahead planner that uses kinematic helpers (`arrival_time`, `leave_time`, `v_stop_at`, `approach_speed`) to compute per-vehicle `DrivePlanEntry` sequences within a braking horizon.
- `register_approaches`: converts planned approach requests into `link_states` to coordinate conflicting movements.
- `advance_traffic_lights`: update light phases and compute `green_links` that mark which movements are permitted.
- `execute_movements` / `execute_vehicle`: compute safe speeds considering leader vehicles and link openness, move vehicles along lanes, and enqueue `PendingTransfer`s when crossing links.

Testing and extension
- The engine is organized to allow targeted unit tests for drive-plan construction, traffic-light progression, and move execution. Extension points include alternative kinematic models, improved link-directory structures for faster lookups, and pluggable traffic controllers.

Diagrams and flows in the original auto page have been condensed into this explanatory overview.

Function / method parameter details
- `SimulationEngine::new(config: SimulationConfig, vehicles: Vec<Vehicle>) -> Self`: `config` holds timing and `map`; `vehicles` is the initial vehicle population. Initializes `vehicles_by_lane` and traffic-light runtime state.
- `SimulationEngine::step(&mut self)`: advances the engine by `config.time_step` and performs the pipeline described above. No parameters; operates on `self` and internal state.
- `SimulationEngine::run(&mut self)`: synchronous loop calling `step()` until `current_time >= end_time` (mainly used for tests or non-async contexts).
- Helper expectations: `rebuild_drive_plan(vidx)` expects `vidx` to be the index into `self.vehicles` and returns a vector of `DrivePlanEntry` describing lookahead movements.
