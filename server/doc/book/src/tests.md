# Tests et stabilisation

## Tests bus

`bus_waypoint_tests.rs` couvre la construction des waypoints et les cas limites les plus simples :

- une route a deux noeuds ne produit pas de waypoint intermediaire
- une route a trois noeuds produit un seul waypoint
- la destination finale reste en dehors de la liste des waypoints

Ces tests servent surtout de garde-fou pour la logique de routage introduite par la branche.

## Corrections front

Le commit `fix test front` a resserre la synchronisation de l'etat du contexte d'edition apres l'arrivee des panneaux bus.
Le commit `fix npm run lint` a elimine les derniers imports et morceaux de code devenus inutiles.

## Limite restante

La couverture reste tres orientee unite et etat local. Il n'y a pas encore de scenario bout-en-bout complet dans la doc pour valider la creation d'une ligne, son affichage et sa lecture cote serveur en une seule execution.