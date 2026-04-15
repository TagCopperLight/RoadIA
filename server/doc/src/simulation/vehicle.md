# Vehicle model

Fichier: `src/simulation/vehicle.rs`.

Types clés:
- `VehicleSpec` : paramètres physiques (accélération max, longueur, confort de décélération, etc.).
- `Vehicle` : état courant, `trip`, `path`, `position_on_lane`, `velocity`, `drive_plan`.
- `LaneId` : identifie une lane normale (`Normal(EdgeIndex, lane_id)`) ou interne (`Internal(intersection_id, internal_id)`).

Fonctions importantes:
- `fastest_path(map, source, destination)` : A* pondéré par longueur/speed_limit.
- `update_path(&mut self, map)` : met à jour le `path` d'un véhicule.
- `compute_acceleration(...)` : calcule l'accélération cible basée sur l'IDM-like logic.
- `get_coordinates`, `get_heading` : positionnement pour rendu / export.

Détails et schémas champ-par-champ

`VehicleSpec` fields:
- `kind`: `Car | Bus` — véhicule logique.
- `max_speed`: vitesse maximale (m/s).
- `max_acceleration`: acceleration maximale (m/s²).
- `comfortable_deceleration`: décélération confortable (m/s²).
- `reaction_time`: temps de réaction (s).
- `length`: longueur du véhicule (m).

`DrivePlanEntry` (généré par `rebuild_drive_plan`):
- `link_id` (u32): id du mouvement interne attendu.
- `lane_id` (LaneId): lane d'origine (EdgeIndex + lane index) ou internal.
- `via_internal_lane_id` (u32): id de l'`InternalLane` traversée à la jonction.
- `junction_id` (u32): id de l'intersection concernée.
- `v_pass` (f32): vitesse cible pour traverser la jonction (m/s).
- `v_wait` (f32): vitesse à maintenir en attente (m/s).
- `arrival_time`, `leave_time` (f32): fenêtres temporelles estimées (s, temps simulé).
- `distance` (f32): distance cumulée jusqu'à ce point (m).
- `set_request` (bool): si `true`, le véhicule inscrit une approche sur ce link.

`compute_acceleration(desired_velocity, minimum_gap, vehicle_ahead_distance, vehicle_ahead_velocity) -> f32`:
- `desired_velocity` (m/s): vitesse visée par la route / limite.
- `minimum_gap` (m): écart minimal entre véhicules (si 0 converti en 0.1 internal).
- `vehicle_ahead_distance` (m): distance disponible jusqu'au véhicule précédent (ou inf).
- `vehicle_ahead_velocity` (m/s): vitesse précédente du véhicule devant.

Retour: accélération (m/s²). Implémentation: term free-road + terme de freinage calculé via l'IDM-like formule; retourne `-comfortable_deceleration` si `vehicle_ahead_distance <= 0`.

Exemple numérique concis:
```rust
let accel = vehicle.compute_acceleration(13.9, 2.0, 10.0, 5.0);
// accel en m/s^2; valeur positive => accélération, négative => freinage
```
