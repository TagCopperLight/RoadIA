# `src/map/editor.rs`

Overview
- Utilities for programmatic and interactive map editing: add/move/delete intersections and roads, create roundabouts and traffic light controllers, and validate edits before applying them to the `Map` graph.

Capabilities
- Node operations: `add_node`, `delete_node`, `move_node`, `update_node` — create or mutate intersections while maintaining id maps and recalculating affected geometry.
- Road operations: `add_road`, `add_two_way_road` (via higher-level helpers), `delete_road`, `update_road` — manage edges and lane counts; `move_node` triggers length recomputation for connected roads.
- Higher-level helpers: `add_roundabout` constructs a ring of nodes/roads; `add_traffic_light_controller` registers a controller with phase definitions (sets of link ids with durations and offsets).

Error handling
- Editor functions return `Result` with descriptive errors when referenced ids are missing or geometry constraints are violated. Consumers should display or log these messages for debugging.

Notes
- Editor functions are used by WebSocket map-edit commands and by offline map generation scripts; they keep the `Map` in a consistent state expected by the planner and engine.

Function parameter summary

- `add_node(map: &mut Map, x: f32, y: f32, kind: IntersectionKind) -> u32`
	- `x, y`: coordonnées monde (m)
	- `kind`: classification (`Habitation`, `Workplace`, `Intersection`)
	- Retour: nouvel `node id` (u32)
	- Exemple JSON (WS `AddNode`):

```json
{ "id": "AddNode", "data": { "x": 123.4, "y": 56.7, "kind": "Intersection" } }
```

- `delete_node(map: &mut Map, id: u32) -> Result<(), String>`
	- Supprime le noeud et son index s'il existe.
	- Erreurs typiques: `Node <id> not found`.

- `move_node(map: &mut Map, id: u32, x: f32, y: f32) -> Result<(), String>`
	- Met à jour la position et recalcule les longueurs des routes entrantes/sortantes.
	- Retombe: `Ok(())` ou `Err("Node <id> not found")`.
	- Exemple JSON (WS `MoveNode`):

```json
{ "id": "MoveNode", "data": { "id": 42, "x": 130.0, "y": 60.2 } }
```

- `update_node(map: &mut Map, id: u32, kind: IntersectionKind) -> Result<(), String>`
	- Change `map.graph[idx].kind`.

- `add_road(map: &mut Map, from_id: u32, to_id: u32, lane_count: u8, speed_limit: f32) -> Result<u32, String>`
	- Calcule `length` à partir des coordonnées et des `radius` des intersections.
	- Vérifie qu'il n'existe pas déjà une arête identique.
	- Retour: `Ok(road_id)` ou `Err("Road ... already exists" | "Node ... not found")`.
	- Exemple JSON (WS `AddRoad`):

```json
{ "id": "AddRoad", "data": { "from_id": 10, "to_id": 11, "lane_count": 2, "speed_limit": 13.9 } }
```

- `delete_road(map: &mut Map, id: u32) -> Result<(), String>`
	- Recherche `edge_idx = map.find_edge(id)` et supprime l'arête.

- `update_road(map: &mut Map, id: u32, speed_limit: f32) -> Result<(), String>`
	- Met à jour `road.speed_limit` en le clampant entre `1.0` et `MAX_SPEED`.

- `add_roundabout(map, center_x, center_y, ring_radius, num_arms, ring_speed_limit, ring_lane_count) -> RoundaboutHandle`
	- Paramètres:
		- `center_x, center_y` : centre du rond-point
		- `ring_radius` : rayon interne (doit respecter un minimum calculé)
		- `num_arms` : >= 3
		- `ring_speed_limit` : limite en m/s
		- `ring_lane_count` : >= 1
	- Retourne `RoundaboutHandle { ring_node_ids, ring_road_ids }`.
	- Exemples/contraintes: la fonction assert! sur `num_arms`, `ring_radius` et `ring_lane_count`.

- `add_traffic_light_controller(map, intersection_id, phases) -> Result<TrafficLightControllerHandle, String>`
	- `phases`: `Vec<(Vec<u32>, f32, f32)>` où chaque tuple est `(green_link_ids, green_duration, yellow_duration)`.
	- Effet: marque les `link.link_type = LinkType::TrafficLight` pour link ids fournis et crée un `TrafficLightController` stocké dans `map.traffic_lights`.
	- Retourne `TrafficLightControllerHandle { controller_id }`.

## Erreurs fréquentes & validations

- `Node <id> not found` — référence à un `id` inexistant dans `node_index_map`.
- `Road ... already exists` — tentative d'ajout d'une route dupliquée.
- `Traffic light controller must have at least one phase` — `phases.is_empty()`.

## Notes d'implémentation

- `add_roundabout` crée les noeuds du ring puis les routes chordales;
	appeler ensuite `roundabout::finalize_roundabout_links` pour régler les priorités (yield) entre entrées et anneau.
- `add_traffic_light_controller` incrémente `map.next_controller_id` et insère le contrôleur dans `map.traffic_lights`.

