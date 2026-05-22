# Map model

Fichier: `src/map/model.rs`.

Ce module est la source de vérité de la topologie routière. Il contient la carte complète, ses réglages globaux, les identifiants utilisés par le moteur et les helpers de persistance.

## `MapSettings`

Les réglages de la carte contrôlent à la fois la génération des véhicules, le calcul du score et les paramètres de budget du projet.

| Champ | Rôle | Valeur par défaut |
|---|---|---|
| `vehicle_count` | Nombre de véhicules générés au départ. | 500 |
| `simulation_duration` | Durée maximale de la simulation. | 600.0 s |
| `max_budget` | Budget de référence pour l'infrastructure. | 750000000 |
| `base_cost_per_meter` | Coût d'une route par mètre. | 500 |
| `intersection_cost` | Coût d'une intersection. | 50000 |
| `habitation_cost` | Coût associé aux zones d'habitation. | 150000 |
| `workplace_cost` | Coût associé aux zones de travail. | 200000 |
| `time_weight` | Poids de la dimension temps dans le score. | 0.4 |
| `success_weight` | Poids du taux de réussite. | 0.2 |
| `pollution_weight` | Poids de la pollution. | 0.2 |
| `infrastructure_weight` | Poids de l'infrastructure. | 0.2 |

## `SavedBusLine`

Ligne de bus persistée sur disque.

- `id` : identifiant de ligne.
- `name` : nom lisible.
- `stop_node_ids` : identifiants des arrêts.

## `Map`

La carte contient:

- un graphe orienté `Intersection -> Road`;
- une table de correspondance entre identifiants publics et indices `petgraph`;
- des compteurs d'identifiants pour les noeuds, routes, liens et contrôleurs;
- les contrôleurs de feux;
- le nom de la carte et ses réglages;
- les lignes de bus sauvegardées.

### Champs principaux

| Champ | Rôle |
|---|---|
| `graph` | Graphe routier principal. |
| `node_index_map` | Correspondance `id -> NodeIndex`. |
| `next_node_id` | Prochain identifiant de noeud à attribuer. |
| `next_edge_id` | Prochain identifiant de route à attribuer. |
| `next_link_id` | Prochain identifiant de lien interne. |
| `next_controller_id` | Prochain identifiant de contrôleur de feux. |
| `traffic_lights` | Contrôleurs de feux actifs. |
| `name` | Nom lisible de la carte. |
| `settings` | Réglages globaux. |
| `bus_lines` | Lignes de bus persistées. |
| `next_bus_line_id` | Prochain identifiant de ligne de bus. |

### Méthodes utiles

#### add_intersection
Ajoute un nœud à la carte.

- **Entrées** :
    - `kind: IntersectionKind` : Le type de nœud (Habitation, Workplace, etc.).
    - `x, y: f32` : Les coordonnées géographiques projetées.
- **Action** : Insère un nouveau nœud dans le graphe et met à jour la table de correspondance des IDs.
- **Retour** : `u32` (l'identifiant public du nœud).

#### add_road
Ajoute une route orientée entre deux intersections.

- **Entrées** :
    - `from, to: u32` : Identifiants publics des nœuds de départ et d'arrivée.
    - `lane_count: u8` : Nombre de voies.
    - `speed_limit: f32` : Vitesse limite en m/s.
    - `length: f32` : Longueur en mètres.
- **Action** : Crée une arête dans le graphe reliant les deux nœuds.
- **Pré-conditions** : Les nœuds `from` et `to` doivent exister.
- **Retour** : `u32` (l'identifiant public de la route).

#### add_two_way_road
Ajoute une liaison bidirectionnelle.

- **Action** : Appelle `add_road` deux fois avec les directions inversées.
- **Entrées** : `from, to: u32`, `lane_count: u8`, `speed_limit: f32`, `length: f32`.
- **Retour** : `(u32, u32)` (les identifiants des deux routes créées).

#### find_node / find_edge
Convertit un identifiant public (u32) en un index interne (`NodeIndex` ou `EdgeIndex`).

- **Entrées** : `id: u32`.
- **Retour** : `Option<NodeIndex>` ou `Option<EdgeIndex>`.

#### neighboring_intersections
Liste les nœuds voisins accessibles directement depuis un nœud donné.

- **Entrées** : `source: NodeIndex`.
- **Retour** : `Vec<NodeIndex>`.

#### intersection_neighbor_distance
Donne la longueur de la route reliant deux intersections adjacentes.

- **Entrées** : `source`, `destination`.
- **Retour** : `Option<f32>`.

#### intersections_euclidean_distance
Calcule la distance à vol d'oiseau entre deux intersections.

- **Entrées** : `source`, `destination`.
- **Retour** : `f32` (mètres).

#### retain_largest_component
Nettoie la carte pour ne garder que la partie principale.

- **Action** : Identifie les composantes faiblement connexes du graphe (en ignorant l'orientation des routes) et supprime tous les nœuds et arêtes n'appartenant pas à la plus grande composante.
- **Post-conditions** : Garantit que le réseau routier est d'un seul bloc, évitant les véhicules bloqués sur des fragments isolés.

#### save / load
Gère la persistance de la carte au format JSON.

- **Entrées** : `path: P` (chemin du fichier).
- **Retour** : `Result`.

### Coordinates

Structure légère utilisée pour les positions dans le monde simulé.

- `x` : coordonnée horizontale.
- `y` : coordonnée verticale.

## Lecture fonctionnelle

- `Map` décrit la topologie.
- `MapSettings` décrit comment cette topologie doit être utilisée dans la simulation.
- `SavedBusLine` permet de persister les transports publics sans les confondre avec la structure routière.
- `Coordinates` est le type élémentaire réutilisé partout où une position est nécessaire.
