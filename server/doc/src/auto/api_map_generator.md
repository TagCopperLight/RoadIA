<!-- Auto documentation: extracted from server/src/api/runner/map_generator.rs -->

# `api::runner::map_generator`

Fonctions utilitaires pour construire des `Map` et une flotte de `Vehicle` à usage de tests et pour l'initialisation par défaut.

## Principales fonctions exportées

- `create_osm_map<P: AsRef<Path>>(path: P) -> Result<Map, osm_parser::OsmParseError>`
	- Charge un `.osm.pbf` via `osm_parser::parse_osm_pbf`.
	- Retient seulement la plus grande composante connectée (`retain_largest_component`).
	- Tagge les noeuds en `Habitation`, `Workplace` ou `Intersection` selon degrés entrant/sortant.
	- Construit les intersections (`intersection::build_intersections`) et renvoie `Map` prêt à l'emploi.

- `create_random_vehicles(map: &Map, count: usize) -> Vec<Vehicle>`
	- Cherche `Habitation` et `Workplace` nodes dans le `Map`.
	- Pour `count` véhicules: choisit aléatoirement origine/destination et crée `Vehicle` avec `VehicleSpec` par défaut (ex: `Car`, vitesse nominale 40.0 etc.).
	- Retourne vecteur de `Vehicle`.

- `create_connected_map(num_nodes, width, height) -> Map`
	- Génère `num_nodes` positions aléatoires en respectant un espacement minimal.
	- Construit un arbre couvrant minimal (MST-like) pour assurer connectivité.
	- Ajoute connexions additionnelles entre voisins proches pour créer cycles.
	- Retour: `Map` avec intersections et routes bidirectionnelles.

- `create_traffic_light_test_map()`, `create_roundabout_test_map()`, `create_multilane_test_map()`, `create_intersection_test_map()`, `create_one_intersection_congestion_map()`
	- Fonctions utilitaires qui construisent cartes de test (rond-point, feux, multilane, congestion) prêtes à l'emploi.

## Détails & comportements

- Randomness: utilise `rand::random_range` pour positions, choix d'origines/destinations et vitesses; résultats non déterministes.
- `create_osm_map` modifie `map.graph` pour catégoriser les noeuds et appelle `intersection::build_intersections`.
- `create_random_vehicles` retourne un vecteur vide si il manque des `Habitation` ou `Workplace`.

## Exemples

```ignore
// Charger carte OSM et créer 500 véhicules aléatoires
let map = create_osm_map("data/lannion.osm.pbf")?;
let vehicles = create_random_vehicles(&map, 500);

// Map de test simple
let map2 = create_intersection_test_map();
```

## Recommandations

- Pour des tests reproductibles, remplacer les appels RNG par un générateur initialisé avec `SeedableRng`.
- `create_osm_map` appelle `retain_largest_component()` — utile en environnement OSM bruyant.
- Ces utilitaires sont destinés aux scénarios de test/démo; pour production, charger des cartes validées et configurer `VehicleSpec` explicitement.

---

Je peux convertir ces pages `auto` en pages plus structurées (avec sections `Examples`, `Errors`, `Notes`) et lier les cartes de test depuis l'index si tu veux.

# `src/api/runner/map_generator.rs`

Overview
- Map and vehicle generation utilities used to create test maps, load OSM datasets, and produce randomized vehicle populations for simulations.

Capabilities
- `create_osm_map(path)`: parse an OSM PBF file, build intersections, and produce a `Map` trimmed to its largest connected component.
- `create_random_vehicles(map, count)`: sample origin/destination pairs from the map and create a list of `Vehicle` instances.
- `create_connected_map`, `create_one_intersection_congestion_map`, `create_intersection_test_map`, `create_traffic_light_test_map`, `create_roundabout_test_map`, `create_multilane_test_map`: convenience generators that produce deterministic test maps useful for unit tests and demonstrations.

Notes
- `create_osm_map` is used by `SimulationInstance::new_default()` to load the provided `data/lannion.osm.pbf` dataset when available.
- These utilities focus on practical test-case generation rather than full fidelity OSM imports; they prepare maps suitable for routing and simulation within this project.
