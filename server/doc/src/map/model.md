# Map model

Fichier: `src/map/model.rs`.

Structures principales:

- `Map` : conteneur principal contenant un `petgraph::Graph<Intersection, Road>`, indices, compteurs d'ID et `traffic_lights`.
  - Méthodes utiles: `new()`, `add_intersection()`, `add_road()`, `add_two_way_road()`, `find_node()`, `find_edge()`, `retain_largest_component()`.

- `Coordinates` : structure légère `x, y` pour positions.

Comportement important:
- `retain_largest_component()` nettoie les fragments OSM isolés en ne conservant que la composante la plus grande (utile après parsing OSM).

Utilisation:
- Le module est la source de vérité pour la topologie routière utilisée par le simulateur.
