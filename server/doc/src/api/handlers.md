# HTTP handlers

Fichier principal: `src/api/runner/handlers.rs`.

Ce chapitre documente l'état partagé du serveur HTTP et les routes qui créent, modifient et chargent les simulations. Les handlers manipulent surtout les instances en mémoire, les fichiers de carte persistés sur disque, et la configuration de la carte.

## État partagé

### `AppState`

Contient toutes les simulations actives, indexées par `Uuid`. Chaque entrée pointe vers une `SimulationInstance` partagée entre les routes HTTP et le WebSocket.

## Requêtes JSON

### `CustomMapRequest`

Décrit une zone géographique à extraire depuis OpenStreetMap.

- `min_lat`, `min_lon` : coin sud-ouest du rectangle.
- `max_lat`, `max_lon` : coin nord-est du rectangle.

### `SaveMapRequest`

Requête de sauvegarde d'une carte existante.

- `uuid` : identifiant de la simulation à sauvegarder.
- `token` : jeton d'accès associé.
- `name` : nom à attribuer à la carte.
- `file_uuid` : identifiant du fichier JSON de destination, s'il existe déjà.

### `LoadMapRequest`

Charge une carte enregistrée à partir de son identifiant de fichier.

### `RenameMapRequest`

Renomme une carte persistée sans recréer de simulation.

### `DeleteMapRequest`

Supprime le fichier JSON d'une carte persistée.

## Routes et responsabilités

| Route | Fonction | Rôle |
|---|---|---|
| `POST /api/simulations` | `create_simulation_handler` | Crée une simulation par défaut à partir de `data/lannion.osm.pbf`. |
| `POST /api/custom_map` | `create_custom_simulation_handler` | Interroge Overpass, convertit la zone demandée et crée une simulation sur mesure. |
| `POST /api/simulations/save-map` | `save_map_handler` | Sauvegarde la carte active sur disque. |
| `POST /api/simulations/load-map` | `load_map_handler` | Charge une carte persistée et crée une nouvelle simulation associée. |
| `GET /api/maps` | `list_maps_handler` | Liste les cartes sauvegardées sur disque. |
| `POST /api/maps/rename` | `rename_map_handler` | Renomme une carte persistée. |
| `POST /api/maps/delete` | `delete_map_handler` | Supprime une carte persistée. |
| `GET /api/simulations/:uuid/settings` | `get_simulation_settings_handler` | Lit les réglages de simulation depuis la carte active. |
| `POST /api/simulations/:uuid/settings` | `update_simulation_settings_handler` | Met à jour les réglages de score et de carte, puis les persiste si nécessaire. |
| `run` | Démarrage du serveur | Construit le routeur, installe le CORS et lance Axum. |

## Comportement des handlers

### create_simulation_handler
Crée une simulation par défaut.
- **Action** : Charge la carte de Lannion et initialise une nouvelle instance de simulation.

### create_custom_simulation_handler
Récupère une zone géographique d'OSM et crée une simulation.
- **Entrées** : `State(state)`, `Json(payload)` (coordonnées).
- **Action** : Télécharge les données via Overpass API, les convertit, et initialise une nouvelle instance de simulation.
- **Retour** : `Json` contenant l'UUID et le jeton de la nouvelle instance.

### save_map_handler / load_map_handler
Gèrent la persistance des cartes sur le serveur.
- **Action** : Sauvegardent l'état actuel d'une simulation dans un fichier JSON ou chargent un fichier existant pour créer une nouvelle simulation.

### rename_map_handler / delete_map_handler
Opérations de maintenance sur les fichiers de cartes.
- **Action** : Renomment ou suppriment physiquement les fichiers JSON du répertoire de données.

### list_maps_handler
Liste toutes les cartes enregistrées sur le serveur.
- **Retour** : `Json` contenant la liste des noms et IDs de fichiers.

### get_simulation_settings_handler / update_simulation_settings_handler
Accèdent aux réglages de la carte active.
- **Action** : Permettent de consulter ou de modifier les poids du score et les paramètres de budget en cours de simulation.

### `run`

Construit le `Router`, installe les routes HTTP, configure le CORS avec `ALLOWED_ORIGINS`, crée l'état partagé initial puis lance le serveur sur `0.0.0.0:8080`.

## À retenir

- Les handlers ne contiennent pas la logique de simulation elle-même.
- Ils servent surtout d'interface de création, de persistance et de configuration.
- Les validations d'entrée sont importantes, en particulier pour les limites géographiques et le calcul des poids du score.