# Générateur de Cartes et Véhicules

Ce module fournit des fonctions utilitaires pour créer des environnements de simulation complets, que ce soit à partir de données réelles (OSM) ou pour des tests spécifiques.

## Fonctions

### create_osm_map
Charge une carte depuis un fichier OSM PBF et configure les types d'intersections.

- **Entrées** :
    - `path: P` : Chemin vers le fichier `.osm.pbf`.
- **Action** :
    1. Analyse le fichier PBF via le module `osm_parser`.
    2. Ne conserve que la plus grande composante connexe de la carte pour éviter les fragments isolés.
    3. Assigne automatiquement des rôles aux nœuds :
        - `Habitation` : Nœuds terminaux avec uniquement des sorties.
        - `Workplace` : Nœuds terminaux avec uniquement des entrées.
        - `Intersection` : Nœuds avec un degré élevé.
    4. Réduit le nombre d'habitations et de lieux de travail pour une distribution plus réaliste (environ 1/3 conservés).
    5. Appelle `build_intersections` pour générer la logique interne des carrefours.
- **Retour** : `Result<Map, OsmParseError>`.

### create_random_vehicles
Génère une liste de véhicules avec des trajets aléatoires mais valides.

- **Entrées** :
    - `map: &Map` : La carte de simulation.
    - `count: usize` : Le nombre de véhicules à créer.
- **Action** :
    1. Identifie tous les nœuds de type `Habitation` et `Workplace`.
    2. Utilise un parcours en largeur (BFS) pour identifier les paires (départ, arrivée) qui sont réellement connectées.
    3. Crée des véhicules avec des spécifications variées (Voitures, Bus) et des types de moteurs différents.
    4. Répartit les départs sur 24h en utilisant des **plans de trajets pendulaires (`CommutePlan`)** pour simuler un cycle de vie complet.
- **Retour** : `Vec<Vehicle>`.

### create_connected_map
Génère une carte aléatoire structurée sous forme de grille ou de graphe connecté.

- **Entrées** :
    - `num_nodes` : Nombre de nœuds souhaités.
    - `width, height` : Dimensions de la zone.
- **Action** : Place les nœuds aléatoirement et tente de créer des connexions pour former un réseau routier cohérent.
- **Retour** : `Map`.

## Cartes de Test (Scénarios)

Le module contient plusieurs fonctions créant des micro-cartes pour tester des comportements spécifiques :

- **create_one_intersection_congestion_map** : Une seule intersection avec beaucoup de trafic pour tester les bouchons.
- **create_intersection_test_map** : Une croix standard (4 bras) pour tester les priorités à droite.
- **create_traffic_light_test_map** : Une intersection équipée de feux de signalisation.
- **create_roundabout_test_map** : Un rond-point fonctionnel.
- **create_multilane_test_map** : Routes à plusieurs voies pour tester les changements de file et les dépassements.

### link_ids_for_arm
Fonction utilitaire pour récupérer les identifiants de liens entre deux nœuds d'une intersection.

- **Entrées** : `map`, `from_id`, `to_id`.
- **Retour** : `Vec<u32>` (liste des IDs de liens).
