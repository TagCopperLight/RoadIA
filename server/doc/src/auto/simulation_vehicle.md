<!-- Auto documentation: extracted from server/src/simulation/vehicle.rs -->

# `simulation::vehicle`

Décrit les véhicules, leurs paramètres, états et utilitaires de chemin les plus
importants.

## Types clés

- `VehicleKind` — `Car | Bus`.

- `VehicleSpec` — spécification physique:
	- `kind: VehicleKind`
	- `max_speed: f32` (m/s)
	- `max_acceleration: f32` (m/s²)
	- `comfortable_deceleration: f32` (m/s²)
	- `reaction_time: f32` (s)
	- `length: f32` (m)

- `TripRequest` — origine/destination (`NodeIndex`) et `departure_time`.

- `LaneId` — identifie une voie: `Normal(EdgeIndex, lane_id)` ou `Internal(intersection_id, internal_lane_id)`.

- `VehicleState` — `WaitingToDepart | OnRoad | Arrived`.

- `DrivePlanEntry` — plan étape par étape pour traverser une jonction.

- `Vehicle` (champs principaux):
	- `id`, `spec`, `trip`, `state`
	- `path`, `path_index` — chemin calculé (sequence de `NodeIndex`)
	- `position_on_lane`, `velocity`, `previous_velocity`
	- `current_lane`, `drive_plan`, `registered_link_ids`
	- metrics: `waiting_time`, `impatience`, `emitted_co2`, `distance_traveled`, `arrived_at`

## Fonctions importantes

- `fastest_path(map, source, destination) -> Option<Vec<NodeIndex>>`
	- A* pondéré par `edge.length / edge.speed_limit` (temps minimal estimé).

- `Vehicle::new(id, spec, trip) -> Vehicle` — initialise état par défaut.

- `update_path(&mut self, map)` — met à jour `self.path` via `fastest_path`.

- `compute_acceleration(&self, desired_velocity, minimum_gap, vehicle_ahead_distance, vehicle_ahead_velocity) -> f32`
	- Implémente un contrôle de type IDM (Intelligent Driver Model)-like:
		- `free_road_acc` calculé depuis `max_acceleration` et `previous_velocity`
		- calcule `s` = spacing minimal requis (réaction_time, gap, s_delta)
		- pénalise l'accélération en proportion de `(s / vehicle_ahead_distance)^2`

- `get_coordinates(&self, map) -> Coordinates` — calcule les coordonnées monde du véhicule
	en fonction de `current_lane` (internal lane vs normal) et `position_on_lane`.

- `get_heading(&self, map) -> f32` — angle en radians (utilisé pour rendu)

## Exemples

```ignore
let spec = VehicleSpec::new(VehicleKind::Car, 40.0, 4.0, 3.0, 1.0, 4.5);
let trip = TripRequest { origin, destination, departure_time: 0.0 };
let mut v = Vehicle::new(1, spec, trip);
v.update_path(&map);
let accel = v.compute_acceleration(30.0, 2.0, 10.0, 20.0);
```

## Remarques

- `compute_acceleration` retourne une accélération nette (m/s²) ; l'engine
	applique l'intégration Euler explicite: `v += a * dt`.

---

# `src/simulation/vehicle.rs`

Overview
- Models `Vehicle` runtime state and supporting types used by the simulation engine. Contains vehicle specifications, trip requests, current kinematic state, and utilities for pathfinding and coordinate conversion.

Key concepts
- `VehicleSpec`: physical and behavioral parameters (max speed, acceleration, reaction time, length) that parameterize per-vehicle dynamics.
- `TripRequest`: origin/destination and intended departure time.
- `VehicleState`: lifecycle states (waiting, on-road, arrived) tracked by the engine.
- `DrivePlanEntry`: internal plan items produced by the planner describing expected lane/link traversals and target speeds.

Important functions
- `fastest_path(map, source, destination)`: computes a shortest/fastest node path used to initialize vehicle routes.
- `Vehicle::update_path`: updates a vehicle's route using pathfinding results.
- `Vehicle::compute_acceleration`: IDM-like car-following acceleration computation used by the engine to determine per-step acceleration given a leader and desired speed.
- Coordinate helpers (`get_coordinates`, `get_heading`) translate vehicle lane/position into world coordinates for rendering or serialization.

Notes for reviewers
- `compute_acceleration` implements safety-critical logic (gap handling, comfortable braking) and benefits from targeted unit tests. `get_coordinates` must handle both normal edge lanes and internal intersection lanes consistently.

Parameter details
- `fastest_path(map: &Map, source: NodeIndex, destination: NodeIndex) -> Option<Vec<NodeIndex>>`: `map` is the road network; `source` and `destination` are `petgraph::graph::NodeIndex` values. Returns `Some(path)` when a route exists.
- `Vehicle::new(id: u64, spec: VehicleSpec, trip: TripRequest) -> Self`: `id` unique per-vehicle, `spec` vehicle physical/behavioral parameters, `trip` origin/destination/departure time.
- `Vehicle::update_path(&mut self, map: &Map)`: recomputes `self.path` using `fastest_path`; updates internal path_index and related state.
- `Vehicle::compute_acceleration(&self, desired_velocity: f32, mut minimum_gap: f32, vehicle_ahead_distance: f32, vehicle_ahead_velocity: f32) -> f32`: parameters are current target speed, minimum allowed gap, distance to vehicle ahead, and that vehicle's velocity. Returns scalar acceleration (m/s^2) to apply this timestep.
- `Vehicle::get_coordinates(&self, map: &Map) -> Coordinates`: returns world `(x,y)` for current lane and `position_on_lane`.
- `Vehicle::get_heading(&self, map: &Map) -> f32`: returns heading in radians (or project convention) based on current segment direction.
- `Vehicle::get_current_node() / get_next_node() -> NodeIndex`: helper accessors for planner logic.
- `Vehicle::get_current_road(&self, map: &Map) -> Option<EdgeIndex>`: returns the current road's `EdgeIndex` when on a regular edge; `None` for internal lanes.
