<!-- Auto documentation: extracted from server/src/map/model.rs -->

# `map::model` (`Map`)

Structure principale représentant la carte (graph de `Intersection` + `Road`).

## Champs principaux

- `graph: Graph<Intersection, Road>` — graphe dirigé Petgraph stockant noeuds et arêtes.
- `node_index_map: HashMap<u32, NodeIndex>` — mapping `node id` → `NodeIndex`.
- `next_node_id`, `next_edge_id`, `next_link_id`, `next_controller_id` — compteurs d'objets uniques.
- `traffic_lights: HashMap<u32, TrafficLightController>` — contrôleurs indexés par id.

## Méthodes publiques utiles

- `new()` — constructeur vide.
- `add_intersection(kind, x, y) -> u32` — crée une intersection et retourne son id.
- `add_road(from, to, lane_count, speed_limit, length) -> u32` — crée une arête dirigée et retourne l'id de route.
- `add_two_way_road(from, to, ...) -> (u32, u32)` — ajoute deux routes opposées et retourne leurs ids.
- `find_node(id) -> Option<NodeIndex>` — recherche `NodeIndex` pour un id.
- `find_edge(id) -> Option<EdgeIndex>` — recherche `EdgeIndex` pour un id.
- `neighbouring_intersections(source) -> Vec<NodeIndex>` — voisins sortants.
- `intersection_neighbor_distance(source, destination) -> Option<f32>` — longueur de l'arête entre deux noeuds.
- `intersections_euclidean_distance(source, destination) -> f32` — distance euclidienne entre centres.
- `retain_largest_component()` — supprime composantes déconnectées et reconstruit `node_index_map`.

## Comportements notables

- `retain_largest_component()` :
  - construit une adjacency non dirigée,
  - trouve composantes faibles connexes par BFS,
  - retient la plus grande et supprime les autres (utile pour données OSM bruitées).

- `add_road` panique si `from`/`to` non trouvés (utiliser les vérifications appelantes dans l'éditeur).

## Exemples

```ignore
let mut map = Map::new();
let a = map.add_intersection(IntersectionKind::Habitation, 0.0, 0.0);
let b = map.add_intersection(IntersectionKind::Workplace, 500.0, 0.0);
let (r1, r2) = map.add_two_way_road(a, b, 1, 13.9, 500.0);
```

---
# `src/map/model.rs`

Overview
- The `Map` structure wraps a graph of `Intersection` nodes and `Road` edges and provides high-level operations for building and querying the road network used by the simulation and planner.

Responsibilities
- Construction and modification APIs: add intersections and roads (including two-way roads), maintain unique identifiers for nodes/edges/links, and keep lookup maps in sync.
- Query helpers: translate between project-level numeric ids and `petgraph` indices, compute euclidean and graph distances, and enumerate neighboring intersections.
- Cleanup: `retain_largest_component` removes disconnected OSM fragments to ensure the simulation runs on a single connected component.

Notes
- `Map` methods are designed for both programmatic map generation and interactive editing workflows; changes update internal id maps and graph metadata used by the engine and editor.

Method parameters (summary)
- `Map::new() -> Self`: creates an empty map with initial id counters.
- `add_intersection(&mut self, kind: IntersectionKind, x: f32, y: f32) -> u32`: `kind` is classification, returns new node id.
- `add_road(&mut self, from: u32, to: u32, lane_count: u8, speed_limit: f32, length: f32) -> u32`: creates directed road and returns edge id.
- `add_two_way_road(&mut self, from: u32, to: u32, lane_count: u8, speed_limit: f32, length: f32) -> (u32,u32)`: convenience creating both directions.
- `find_node(&self, id: u32) -> Option<NodeIndex>` / `find_edge(&self, id: u32) -> Option<EdgeIndex>`: id lookups.
- `retain_largest_component(&mut self)`: removes disconnected components (no params).
