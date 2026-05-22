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

## États et dynamique

### `TrafficLightRuntimeState`

Structure interne utilisée par le moteur de simulation pour suivre l'évolution temporelle de chaque contrôleur.

- `phase_index` : index de la phase actuelle dans le vecteur `phases`.
- `time_in_phase` : temps écoulé depuis le début de la phase actuelle (s).

## Cycle de mise à jour

À chaque pas de simulation, le moteur effectue les opérations suivantes :

1. **Incrémentation** : Ajoute `delta_t` au `time_in_phase` de chaque contrôleur.
2. **Transition** : Si le temps dépasse la durée totale de la phase (`green_duration + yellow_duration`), le contrôleur passe à la phase suivante (ou revient à la première) et réinitialise son compteur.
3. **Actualisation des liens** : La liste globale `green_links` est vidée puis remplie avec les identifiants de liens (`green_link_ids`) de toutes les phases actuellement au vert (celles dont le `time_in_phase` est inférieur à `green_duration`).

> **Note sur le jaune** : Pendant la `yellow_duration`, aucun lien n'est ajouté à `green_links` pour ce contrôleur, ce qui interdit tout nouvel engagement dans le carrefour.

## Lecture fonctionnelle

- les phases sont définies en termes de liens internes autorisés;
- le moteur utilise ces phases pour remplir `green_links`;
- le client interagit avec les contrôleurs via l'éditeur pour modifier les durées et les compositions des phases.