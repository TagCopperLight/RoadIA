# Map editor quick examples

Fichier: `src/map/editor.rs` — helper functions pour modifier la carte.

## `add_roundabout` (exemple Rust)

```rust
let handle = add_roundabout(
    &mut map,
    center_x: 100.0,
    center_y: 50.0,
    ring_radius: 30.0,
    num_arms: 4,
    ring_speed_limit: 8.0,
    ring_lane_count: 1,
);
// handle.ring_node_ids and handle.ring_road_ids contiennent les ids créés
```

Contraintes:
- `num_arms >= 3`.
- `ring_radius` doit être supérieur au minimum calculé (fonction de `num_arms`).

## `add_traffic_light_controller` (exemple Rust)

```rust
// phases: Vec<(Vec<link_id>, green_duration_s, yellow_duration_s)>
let phases = vec![ (vec![101, 102], 10.0, 2.0), (vec![201], 8.0, 2.0) ];
let result = add_traffic_light_controller(&mut map, intersection_id, phases);
match result {
    Ok(handle) => println!("controller id = {}", handle.controller_id),
    Err(e) => eprintln!("Failed to add controller: {}", e),
}
```

Notes:
- `add_traffic_light_controller` marque `link.link_type = TrafficLight` pour les link ids fournis et insère le contrôleur dans `map.traffic_lights`.
- Il faut fournir des link ids valides (obtenus après `build_intersections`), ou construire le rond-point avant d'ajouter le contrôleur.

## Utilisation via l'API WebSocket

Les fonctions `add_node`, `add_road`, `move_node`, `update_node`, `delete_node`, `delete_road` sont exposées via les paquets `ClientPacket` (`AddNode`, `AddRoad`, etc.).

Exemple WS pour ajouter un noeud:

```json
{ "id": "AddNode", "data": { "x": 123.4, "y": 56.7, "kind": "Intersection" } }
```

Remarque: `add_roundabout` et `add_traffic_light_controller` ne sont pas directement exposés par les paquets WS standard — utilisez les helpers Rust ou scripts d'initialisation pour construire des configurations complexes.
