# WebSocket protocol

Résumé du protocole échangé via `/ws` (fichier principal: `src/api/websocket.rs`).

Paramètres de connexion (query):
- `uuid`: identifiant de l'instance de simulation (UUID)
- `token`: jeton d'authentification pour l'instance

Paquets clients (`ClientPacket`):
- `StartSimulation`, `StopSimulation`, `ResetSimulation`
- `AddNode { x, y, kind }`, `DeleteNode { id }`, `MoveNode { id, x, y }`, `UpdateNode { id, kind }`
- `AddRoad { from_id, to_id, lane_count, speed_limit }`, `DeleteRoad { id }`, `UpdateRoad { id, speed_limit }`

Paquets serveur (`ServerPacket`):
- `Map { nodes, edges }` — état complet de la carte
- `VehicleUpdate { vehicles, traffic_lights }` — positions et états des véhicules
- `MapEdit { success, error, nodes, edges }` — réponse aux opérations d'édition
- `Score { score, total_trip_time, total_emitted_co2, network_length, total_distance_traveled, success_rate }`

Notes d'implémentation:
- Les messages sont JSON sérialisés via serde/serde_json.
- Les modifications de carte sont refusées si la simulation est en cours.
- La fonction `serialize_map` et `serialize_vehicle` contrôlent le format côté serveur.
