# Intersections

Fichier: `src/map/intersection.rs`.

Concepts principaux:
- `Intersection` contient `internal_lanes` (type `InternalLane`) définissant les mouvements à l'intérieur d'un carrefour.
- `build_intersections(map)` construit les `InternalLane` et `Link` pour chaque noeud du graphe: calcule entrées/sorties, crée `link_id` et identifie les `foe_links` (conflits entre mouvements).
- Fonctions utilitaires: `segments_intersect`, `boundary_point`, `lane_boundary_point`, `is_link_open`, `time_window_conflict`, etc.

Règles de priorité et ouverture de lien:
- `LinkType` influence le comportement (`Priority`, `Yield`, `Stop`, `TrafficLight`).
- `is_link_open` prend en compte le type de lien, l'état des feux (`green_links`), véhicules présents et conflit temporel d'approche.

Ces algorithmes sont centraux pour la logique d'intersection du simulateur.
