<!-- Auto documentation: extracted from server/src/map/osm_parser.rs -->

# `map::osm_parser`

But: convertit un fichier `.osm.pbf` en `Map` utilisable par le moteur.

## Rôle

- Parser deux passes: collecte des *ways* (routes) puis récupération des coordonnées
- Détecte les noeuds d'intersection (nœuds référencés par ≥ 2 ways)
- Scinde chaque `way` en segments reliant intersections et ajoute des `Intersection` + `Road` au `Map`

## API publique

- `parse_osm_pbf<P: AsRef<Path>>(path: P) -> Result<Map, OsmParseError>`
  - Entrée: chemin vers un `.osm.pbf`
  - Retour: `Ok(Map)` construit ou `Err(OsmParseError)`

## Types d'erreur (`OsmParseError`)

- `Io(std::io::Error)`: erreur d'accès fichier
- `Osm(osmpbf::Error)`: erreur du parseur PBF
- `NoHighways`: aucun highway accepté trouvé dans le fichier -> rien à importer

## Comportement et détails importants

- Accepted highway types: `motorway, trunk, primary, secondary, tertiary, residential, unclassified, living_street, service` et variantes `_link`.
- Vitesse interne: stockée en mètres/seconde (m/s). Le parser accepte `"50"` (par défaut km/h), `"30 mph"`, `"walk"` (5 km/h), et `"50 km/h"`.
- Projection: equirectangular centrée sur la moyenne des noeuds — utile pour conversion lat/lon → (x,y) en mètres.
- Seuils et nettoyages:
  - Segments trop courts (< 0.01 m) ignorés.
  - Les extrémités de chaque way sont marquées comme intersections potentielles (incrément de ref count).

## Fonctions internes notables (résumé)

- `collect_highway_data(path)` — Parcourt les ways et extrait `HighwayWay` (refs, type, lanes, maxspeed, oneway). Calcule aussi `node_ref_count`.
- `collect_node_coords(path, needed)` — Parcourt les nodes et récupère lat/lon pour les ids nécessaires.
- `build_map(ways, node_ref_count, node_coords)` — Construit le `Map`:
  - calcule `center_lat/center_lon`
  - split des ways en segments entre intersections
  - calcule longueur via `compute_segment_length` (haversine)
  - projette coords en mètres via `project_coords` et appelle `map.add_intersection(...)`, `map.add_road(...)` ou `map.add_two_way_road(...)` selon `oneway`
- `parse_speed_limit(tag)` — convertit un tag en `Option<f32>` (m/s)
- `default_speed_limit(highway_type)` — valeur par défaut (km/h → m/s)

## Unités

- Distances: mètres (m)
- Vitesse: mètres / seconde (m/s)
- Durées: secondes (si présentes ailleurs)

## Exemples

Rust (usage minimal):

```ignore
use server::map::osm_parser::parse_osm_pbf;
let map = parse_osm_pbf("data/lannion.osm.pbf")?;
println!("Intersections: {}", map.graph.node_count());
```

Exemples de tags OSM pris en charge:

- `highway=secondary`, `maxspeed=50`, `lanes=2`, `oneway=yes`

## Limitations / recommandations

- Le parser suppose des ways orientés: pour les routes bidirectionnelles il appelle `add_two_way_road`.
- Projection equirectangular: précise pour petites zones; pour grandes régions préférez reprojeter correctement.
- Vérifier les `maxspeed` mal formés — `parse_speed_limit` tente plusieurs formats mais peut retourner `None`.

---

Page générée automatiquement à partir du code source — veux-tu que j'ajoute des exemples d'input `.osm.pbf` et des captures d'écran du résultat dans la carte ?
