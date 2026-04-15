<!-- Auto documentation: extracted from server/src/map/intersection.rs -->

# `map::intersection`

Gestion de la construction des `internal_lanes`, `links` et logique d'intersection.

## Types clés

- `IntersectionKind`: `Habitation | Intersection | Workplace`.
- `InternalLane`: `{ id, from_lane_id, to_lane_id, length, speed_limit, entry, exit }`.
- `Intersection`: `{ id, kind, center_coordinates, radius, internal_lanes }`.
- `ApproachData`: `{ arrival_time, leave_time, arrival_speed, leave_speed, will_pass }`.
- `LinkState`: contient `approaching: HashMap<vehicle_id, ApproachData>`.

## `build_intersections(map)`

- Description:
  - Parcourt tous les nœuds marqués `junction` dans le graphe `Map` et construit la structure fine des voies internes (`InternalLane`) et des mouvements (`Link`) qui représentent les traversées possibles de l'intersection.
  - Génère aussi la liste des conflits (`foe_links`) entre mouvements (croisements, merges) qui seront utilisés par l'algorithme de priorité temporelle de l'engine.

- Paramètres:
  - `map: &mut Map` — la carte mutable qui sera enrichie. Le graphe `map.graph` est modifié: les noeuds et arêtes existants restent mais des `InternalLane` et `Link` sont ajoutés et rattachés aux `Lane` correspondantes.

- Effets secondaires:
  - Ajoute/écrit dans: `intersection.internal_lanes`, `lane.links`, `intersection.foe_links`.
  - Calcule géométrie d'entrée/sortie sur le cercle d'intersection (points `entry`/`exit`) en fonction du rayon et des positions des arêtes.

- Exemple d'utilisation (pseudo):

  - Appeler `build_intersections(&mut map)` après la création/édition initiale du graphe pour garantir que les liens internes et conflits existent avant la simulation.


## Fonctions géométriques utilitaires

- `node_coords(map, n)` — coordonnées du noeud.
- `boundary_point(jx,jy,radius, px,py)` — point sur la frontière du cercle d'intersection dirigé vers p.
- `lane_boundary_point(base, perp, lane_idx, lane_width)` — calcule point d'entrée/sortie pour une voie donnée.
- `segments_intersect`, `cross`, `on_segment` — test d'intersection segmentaire pour détecter conflits.

## Logique de priorité et sécurité

- `is_link_open(...)` : évalue si un lien est praticable par un véhicule donné en tenant compte de:
  - `link_type` (Stop/Yield/Priority/TrafficLight),
  - feux (`green_links`),
  - véhicules approchants enregistrés dans `link_states`,
  - conflits internes (veines internes occupées),
  - heuristiques temporelles (`time_window_conflict`) et `ego.impatience`.

### `is_link_open(link, map, link_state, green_links) -> bool`

- Description:
  - Fonction centrale pour décider si un véhicule peut s'engager sur un `Link` (mouvement interne) en tenant compte des règles locales (stop/yield/priority), de l'état des feux et des véhicules approchants.

- Paramètres:
  - `link: &Link` — le mouvement interne évalué (contient les `internal_lane` cible, `link_type`, `id`).
  - `map: &Map` — référence en lecture seule pour accéder à la géométrie et aux entités (voies, intersections, controllers).
  - `link_state: &LinkState` — état runtime contenant pour chaque véhicule approchant une `ApproachData` avec `arrival_time`, `leave_time`, `arrival_speed`, `leave_speed`, `will_pass`.
  - `green_links: &HashSet<u32>` — set d'IDs de liens actuellement ouverts par le contrôleur de feux (green links). Utilisé pour override des stops/yields.

- Retour:
  - `true` si le mouvement est considéré sûr/privilégié pour l'ego; `false` sinon.

- Règles appliquées (résumé):
  - Si `link.link_type` est `TrafficLight` -> autorisé seulement si `green_links` contient `link.id`.
  - Si `Stop` -> on exige qu'aucun `ApproachData` d'un adversaire prioritaire n'entre en conflit temporel.
  - Si `Yield` -> on vérifie `foe_links` et on appelle `time_window_conflict` pour estimer chevauchement de fenêtres entre `ego` et `foes`.
  - Si plusieurs véhicules sont proches, on utilise `will_pass` et `impatience` pour décider d'un tie-breaker; les liens internes occupés bloquent le passage.

- Exemple concret (JSON-style `ApproachData`):

```json
{
  "arrival_time": 12.34,
  "leave_time": 12.90,
  "arrival_speed": 8.5,
  "leave_speed": 7.0,
  "will_pass": true
}
```

- Exemple `LinkState` minimal (conceptuel):

```json
{
  "approaching": {
    "42": { "arrival_time": 12.34, "leave_time": 12.9, "arrival_speed": 8.5, "leave_speed": 7.0, "will_pass": true }
  }
}
```

Notes:
- `time_window_conflict(ego_window, foe_window)` compare les intervalles `[arrival_time, leave_time]` en appliquant une petite marge temporelle (look-ahead) pour réduire les faux positifs.
- `foe_is_to_the_right(ego, foe)` est utilisé pour appliquer la règle de priorité à droite lorsque aucun panneau/feu n'impose une priorité explicite.

## Fonctions auxiliaires

- `foe_is_to_the_right(ego, foe)` — orientation relative pour heuristique priorité à droite.
- `time_window_conflict(...)` — compare fenêtres d'arrivée/départ et look-ahead pour détecter collision temporelle.

---
# `src/map/intersection.rs`

Overview
- Intersection utilities compute internal traversal lanes, link objects for every permitted movement, and conflict relationships used by the engine to decide right-of-way.

Key concepts
- `InternalLane` and `Link` model the fine-grained paths through an intersection; `foe_links` record conflicting movements (crossing/merging) used for temporal conflict checks.
- `LinkState` and `ApproachData` track approaching vehicles' estimated arrival/leave times and intended behavior (`will_pass`) to support time-window conflict resolution.

Important functions
- `build_intersections(map)`: construct internal lanes and links for each junction and populate `foe` relationships.
- `is_link_open(...)`: central right-of-way decision function that considers internal lane occupancy, stop signs, traffic lights, priority rules and temporal conflicts between approaching vehicles.

Notes
- The intersection module concentrates geometric helpers (boundary points, segment intersection tests) and the conflict detection logic. Its outputs drive `rebuild_drive_plan` and `execute_vehicle` behavior in the engine.

Function parameters (key)
- `build_intersections(map: &mut Map)`: consumes a mutable `Map` and populates internal lanes, `Link` objects and `foe_links` for each junction found in the graph.
- `is_link_open(link: &Link, map: &Map, link_state: &LinkState, green_links: &HashSet<u32>) -> bool`: typical parameter set — `link` being tested, the `map` for geometry, runtime `link_state` with approaching vehicles, and `green_links` set produced by traffic light controllers; returns `true` when movement can proceed.
