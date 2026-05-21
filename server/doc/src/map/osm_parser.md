# Analyseur OSM

Le module `osm_parser` est responsable de la lecture des fichiers de données OpenStreetMap (au format PBF) et de leur conversion en une structure de carte (`Map`) utilisable par le moteur de simulation.

## Énumérations

### OsmParseError
Représente les erreurs possibles lors de l'analyse d'un fichier OSM.

- **Variantes** :
    - `Io(std::io::Error)` : Erreur d'entrée/sortie lors de l'ouverture ou de la lecture du fichier.
    - `Osm(osmpbf::Error)` : Erreur spécifique à la bibliothèque `osmpbf` lors du décodage des données.
    - `NoHighways` : Lancée si aucune route circulable (highway) n'est trouvée dans le fichier fourni.

## Fonctions

### parse_osm_pbf
Point d'entrée principal pour charger une carte depuis un fichier PBF.

- **Entrées** :
    - `path: P` : Le chemin vers le fichier `.osm.pbf` (doit implémenter `AsRef<Path>`).
- **Action** :
    1. Effectue une première passe pour collecter les routes et compter les références des nœuds.
    2. Identifie les intersections (nœuds référencés par plusieurs routes ou extrémités de routes).
    3. Effectue une deuxième passe pour récupérer les coordonnées géographiques (latitude/longitude) des nœuds nécessaires.
    4. Construit la structure `Map` en projetant les coordonnées géographiques dans un plan 2D.
- **Pré-conditions** : Le fichier doit être au format PBF valide.
- **Post-conditions** : Renvoie une `Map` peuplée avec les intersections et les routes si succès.
- **Retour** : `Result<Map, OsmParseError>`.

### collect_highway_data
Collecte les données de routes et compte les références des nœuds (Passe 1).

- **Entrées** :
    - `path: &Path` : Chemin du fichier.
- **Action** : Parcourt tous les éléments "Way" du fichier OSM, filtre ceux qui correspondent à des types de routes acceptés et non privés, et extrait leurs métadonnées (vitesse, voies, sens unique).
- **Pré-conditions** : Le fichier doit être accessible.
- **Post-conditions** : Aucune modification d'état global.
- **Retour** : `Result<(Vec<HighwayWay>, HashMap<i64, u32>), OsmParseError>`.

### collect_node_coords
Récupère les coordonnées des nœuds nécessaires (Passe 2).

- **Entrées** :
    - `path: &Path` : Chemin du fichier.
    - `needed: &HashSet<i64>` : Ensemble des IDs de nœuds dont on a besoin.
- **Action** : Parcourt les nœuds du fichier OSM et stocke les coordonnées (lat/lon) de ceux présents dans l'ensemble `needed`.
- **Pré-conditions** : L'ensemble `needed` doit contenir les IDs identifiés lors de la passe 1.
- **Post-conditions** : Aucune.
- **Retour** : `Result<HashMap<i64, NodeCoord>, OsmParseError>`.

### build_map
Construit l'objet `Map` final.

- **Entrées** :
    - `ways: &[HighwayWay]` : Liste des routes collectées.
    - `node_ref_count: &HashMap<i64, u32>` : Nombre de fois que chaque nœud est utilisé.
    - `node_coords: &HashMap<i64, NodeCoord>` : Coordonnées géographiques des nœuds.
- **Action** :
    1. Calcule le centre géographique de la zone pour la projection.
    2. Divise les chemins OSM en segments individuels entre chaque intersection.
    3. Calcule la longueur réelle de chaque segment.
    4. Ajoute les intersections et les routes à l'objet `Map`.
- **Pré-conditions** : Les données fournies doivent être cohérentes.
- **Post-conditions** : Crée une carte centrée sur la zone analysée.
- **Retour** : `Result<Map, OsmParseError>`.

### compute_segment_length
Calcule la longueur totale d'une suite de nœuds en mètres.

- **Entrées** :
    - `node_ids: &[i64]` : Liste des IDs de nœuds formant le segment.
    - `coords: &HashMap<i64, NodeCoord>` : Dictionnaire des coordonnées.
- **Action** : Additionne les distances Haversine entre chaque paire consécutive de nœuds.
- **Retour** : `f32` (distance en mètres).

### haversine_distance
Calcule la distance à vol d'oiseau entre deux points géographiques.

- **Entrées** : `lat1, lon1, lat2, lon2` (f64).
- **Action** : Applique la formule de Haversine pour tenir compte de la courbure de la Terre.
- **Retour** : `f64` (distance en mètres).

### project_coords
Projette des coordonnées géographiques sur un plan 2D (équirectangulaire).

- **Entrées** :
    - `lat, lon` : Point à projeter.
    - `center_lat, center_lon` : Point d'origine (0,0) de la projection.
- **Action** : Convertit les degrés en mètres relatifs au centre. L'axe Y est inversé pour correspondre aux systèmes de coordonnées graphiques (Y croissant vers le bas).
- **Retour** : `(f32, f32)` (x, y en mètres).

### parse_speed_limit
Analyse la balise `maxspeed` d'OSM.

- **Entrées** : `tag: &str`.
- **Action** : Convertit des chaînes comme "50", "30 mph", ou "walk" en une valeur numérique en m/s.
- **Retour** : `Option<f32>`.

### default_speed_limit
Fournit une vitesse par défaut selon le type de route.

- **Entrées** : `highway_type: &str`.
- **Action** : Retourne une vitesse standard (ex: 130 km/h pour une autoroute) convertie en m/s.
- **Retour** : `f32`.
