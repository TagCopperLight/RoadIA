# Runner / Server entrypoints

Fichier principal: `src/api/runner/runner.rs`.

Ce chapitre décrit la gestion des simulations en mémoire, les contrôleurs de démarrage/arrêt et le lancement du serveur HTTP + WebSocket.

## `SimulationController`

Petit contrôleur atomique qui indique si une simulation doit tourner.

- `new()` crée un contrôleur arrêté.
- `start()` active la simulation.
- `stop()` l'arrête.
- `is_running()` lit l'état courant.

## `SimulationInstance`

Une instance correspond à une simulation autonome avec sa carte, son moteur, sa diffusion d'événements et son jeton d'accès.

| Champ | Rôle |
|---|---|
| `token` | Jeton attendu par le WebSocket. |
| `engine` | Moteur actif manipulé par le client et le worker. |
| `initial_engine` | Instantané initial réutilisé pour les calculs de score ou les requêtes globales. |
| `broadcast` | Canal de diffusion des paquets serveur vers tous les clients connectés. |
| `controller` | État marche/arrêt partagé avec le WebSocket. |
| `active_connections` | Nombre de clients WebSocket actuellement connectés. |
| `speed_multiplier` | Accélération de simulation appliquée par le worker. |
| `file_uuid` | Identifiant du fichier de carte persisté, si la simulation vient d'un fichier. |

### Fonctions de `SimulationInstance`

- `new(map)` construit une instance complète, initialise les véhicules, recalcule les chemins et lance un worker asynchrone qui publie les mises à jour.
- `from_file(path, uuid)` charge une carte persistée puis en crée une instance.
- `new_default()` essaie de charger `data/lannion.osm.pbf`; si l'import échoue, la simulation démarre sur une carte vide.

Le worker interne publie périodiquement les véhicules, les feux et le paquet de fin de simulation lorsque tout le monde est arrivé ou lorsque la durée maximale est atteinte.

## `AppState`

État partagé du serveur. Il stocke la table des simulations actives, indexées par `Uuid`.

## `generate_token`

Construit un jeton hexadécimal de 32 caractères à l'aide d'un générateur aléatoire.

## Flux de démarrage

1. `run()` crée l'état partagé vide.
2. `run()` configure les origines autorisées via `ALLOWED_ORIGINS`.
3. `run()` enregistre les routes HTTP et WebSocket.
4. `run()` lie le serveur sur `0.0.0.0:8080`.

## Ce qu'il faut retenir

- `handlers.rs` décide des routes et de leurs validations.
- `runner.rs` porte la vie d'une simulation individuelle.
- Le WebSocket ne manipule pas directement les fichiers de carte; il agit sur une `SimulationInstance` déjà existante.
