# Documentation de la branche feature/bus

Cette branche a ajoute un socle complet pour les bus dans RoadIA, puis l'a branche sur une interface de creation de lignes et de waypoints.

La progression generale est la suivante :
- ajout du type `Bus` cote moteur et premier rendu cote client
- ajout du suivi des bus, des arrets et des routes
- ajout des panneaux de saisie pour les waypoints et les lignes
- stabilisation des tests et nettoyage du lint

## Historique

| Commit | Idee principale |
| --- | --- |
| `56d959f` | `bus back + debut front` : socle bus cote serveur et debut du rendu cote client. |
| `a8ebe4d` | `front des bus, demerde toi julien` : panneaux de creation de lignes et de waypoints. |
| `9f6e44b` | `fix test front` : correction de la synchronisation de l'etat cote interface. |
| `ff0bd93` | `c repare` : consolidation de la simulation bus, du websocket et de l'interface. |
| `1036d27` | `fix npm run lint` : nettoyage final des imports et du code inutile. |

## Ce que couvre cette doc

- le modele serveur des vehicules et des bus
- le transport websocket entre serveur et client
- l'interface d'edition des lignes et des waypoints
- les tests de regression lies au routage bus