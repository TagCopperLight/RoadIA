<!-- Auto documentation: extracted from server/src/simulation/kinematics.rs -->

# `simulation::kinematics`

Fonctions utilitaires physiques et temporelles utilisées par le moteur pour
planifier arrivées, départs et vitesses cibles.

## Unités

- Distances: mètres (m)
- Vitesses: mètres / seconde (m/s)
- Durées: secondes (s)

## Fonctions publiques

- `arrival_time(dist: f32, v0: f32, v1: f32, a_max: f32, d_max: f32) -> f32`
	- Calcule le temps minimal pour parcourir une distance `dist` en partant
		de vitesse `v0` et en arrivant à `v1`, en respectant accélération
		maximale `a_max` et décélération confortable `d_max`.
	- Gère deux cas: accélération nette (v1 >= v0) ou décélération (v1 < v0).
	- Formule: combine phases d'accel/decel et phase de croisière. Retour en secondes.

- `leave_time(t_arrive: f32, lane_len: f32, veh_len: f32, v_arrive: f32, v_leave: f32) -> f32`
	- Estime le temps de sortie d'une voie/interne :
		t_leave = t_arrive + (lane_len + veh_len) / avg_speed
	- `avg_speed` calculé comme moyenne de `v_arrive` et `v_leave` bornée ≥ 0.1 m/s.

- `v_stop_at(dist: f32, d_max: f32) -> f32`
	- Calcul de la vitesse maximale pour s'arrêter sur `dist` avec décélération `d_max`:
		$$v = \sqrt{2\,d_{max}\,dist}$$

- `approach_speed(link_type: &LinkType, road_speed_limit: f32) -> f32`
	- Règle heuristique pour vitesse d'approche selon type de lien:
		- `Priority` → full `road_speed_limit`
		- `Yield` → `0.7 * road_speed_limit`
		- `Stop` → `0.0`
		- `TrafficLight` → `road_speed_limit`

## Remarques

- Ces fonctions sont conçues pour être simples et rapides; elles servent
	à planifier des trajectoires (drive plans) et non pour un modèle de
	physique exhaustif.

---

Exemple: calculer le temps pour parcourir 100 m de 10→20 m/s avec a_max=2 m/s²:

```ignore
let t = arrival_time(100.0, 10.0, 20.0, 2.0, 2.0);
```

# `src/simulation/kinematics.rs`

Overview
- Collection of pure kinematic helpers used by the engine and vehicle models to reason about arrival/leave times, safe stopping speeds and approach speed heuristics for different link types.

Key functions
- `arrival_time(dist, v0, v1, a_max, d_max)`: compute traversal time over a distance while transitioning between speeds under acceleration/deceleration limits.
- `leave_time(...)`: estimate the time a vehicle will leave a junction or lane segment given arrival timing and local speeds.
- `v_stop_at(dist, d_max)`: compute the maximum safe speed to be able to stop within a given distance.
- `approach_speed(link_type, road_speed_limit)`: heuristic target speed depending on link semantics (yield, priority, stop sign, traffic lights).

Notes
- These helpers are deliberately isolated and deterministic to make unit-testing straightforward; they are used by `SimulationEngine` when constructing drive plans and by `Vehicle::compute_acceleration` to enforce safe behavior.

Function details
- `arrival_time(dist: f32, v0: f32, v1: f32, a_max: f32, d_max: f32) -> f32`: compute time to travel `dist` while transitioning from `v0` to `v1` under acceleration/deceleration limits.
- `leave_time(t_arrive: f32, lane_len: f32, veh_len: f32, v_arrive: f32, v_leave: f32) -> f32`: estimate time the vehicle will leave a lane/junction given arrival time and speeds.
- `v_stop_at(dist: f32, d_max: f32) -> f32`: compute max safe speed to be able to stop within `dist` using `d_max` deceleration.
- `approach_speed(link_type: &LinkType, road_speed_limit: f32) -> f32`: heuristic for target approach speed depending on link semantic and speed limit.
