# Map model

Fichier: `src/map/model.rs`.

Structures principales:

- `Map` : conteneur principal contenant un `petgraph::Graph<Intersection, Road>`, indices, compteurs d'ID et `traffic_lights`.
  - Méthodes utiles (signatures et description):
    - `Map::new() -> Map` — constructeur vide.
    - `Map::add_intersection(&mut self, kind: IntersectionKind, x: f32, y: f32) -> u32` — ajoute un noeud, retourne son `id` (u32). `x,y` en unités monde (m).
    - `Map::add_road(&mut self, from: u32, to: u32, lane_count: u8, speed_limit: f32, length: f32) -> u32` — ajoute une arête dirigée entre `from` et `to`. Calcule et stocke un `road id` unique.
    - `Map::add_two_way_road(&mut self, from: u32, to: u32, lane_count: u8, speed_limit: f32, length: f32) -> (u32,u32)` — crée deux routes opposées et retourne leurs ids.
    - `Map::find_node(&self, id: u32) -> Option<NodeIndex>` — récupère l'index interne `NodeIndex` si présent.
    - `Map::find_edge(&self, id: u32) -> Option<EdgeIndex>` — recherche un `EdgeIndex` par `road.id`.
    - `Map::retain_largest_component(&mut self)` — supprime fragments isolés (utile après parsing OSM).

- `Coordinates` : structure légère `x, y` pour positions.

Exemples:

```rust
let mut map = Map::new();
let n = map.add_intersection(IntersectionKind::Intersection, 10.0, 5.0); // returns node id
let r = map.add_road(n, other_n, 2, 13.9, 120.0); // returns road id
```

Comportement important:
- `retain_largest_component()` nettoie les fragments OSM isolés en ne conservant que la composante la plus grande (utile après parsing OSM).

Erreurs courantes / garanties:
- `add_road` panique si `from` / `to` inconnus (la version publique `editor::add_road` gère ces erreurs et retourne `Result`).

Notes sur les unités:
- `speed_limit` : unité interne du projet (assimilée à m/s dans la simulation).
- `length`, `x`, `y`, `radius` : unités en mètres.

Utilisation:
- Le module est la source de vérité pour la topologie routière utilisée par le simulateur.
