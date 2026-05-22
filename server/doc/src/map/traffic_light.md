# Traffic lights

Fichier principal: `src/map/traffic_light.rs`.

Ce chapitre décrit les contrôleurs de feux associés à une intersection et les phases qui règlent les liens autorisés.

## Structures

### `SignalPhase`

Une phase décrit l'état d'un feu tricolore pendant une durée donnée. Le moteur alterne entre les phases séquentiellement.

- `green_link_ids` : liste des liens autorisés (ouverts) pendant cette phase;
- `green_duration` : durée pendant laquelle les liens sont au vert (s);
- `yellow_duration` : durée du jaune (période de transition avant la phase suivante) (s).

### `TrafficLightController`

Contrôleur de feux attaché à une intersection.

| Champ | Rôle |
|---|---|
| `id` | Identifiant du contrôleur. |
| `intersection_id` | Identifiant de l'intersection commandée. |
| `phases` | Séquence des phases de feu. |

### `TrafficLightControllerHandle`

Petit handle renvoyé par les fonctions de création pour référencer un contrôleur récemment créé.

- `controller_id` : identifiant à réutiliser pour la suite.

## Lecture fonctionnelle

- les phases sont définies en termes de liens internes autorisés;
- le moteur utilise ces phases pour remplir `green_links`;
- le client n'interagit pas directement avec la structure interne du contrôleur, seulement via les identifiants de liens et les paquets WebSocket.