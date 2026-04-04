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
