# Interface bus

## Etat de l'interface

`EditModeContext` a ete enrichi pour piloter les outils `waypoints` et `bus`, en plus des outils existants `select`, `addNode` et `addRoad`.
Le contexte stocke aussi `busRoutes` et `selectedBusRoute` pour garder la liste des lignes courantes dans l'UI.

`Toolbar` permet de basculer proprement entre le mode edition et le mode simulation, sans laisser d'etat partiel derriere le changement de mode.

## Panneau des waypoints

`WaypointPanel` permet de choisir un vehicule, puis de construire une liste de noeuds intermediaires avant d'envoyer la modification au serveur.

Le panneau empile les waypoints a partir des clics sur la carte, empeche de dupliquer la destination finale, et reset l'etat local quand l'utilisateur annule ou applique.

## Panneau des lignes de bus

`BusRoutePanel` permet de creer une ligne avec :

- un nom editable
- un point de depart
- zero ou plusieurs waypoints
- un terminus

Le premier noeud clique devient le spawn, les noeuds intermediaires sont traites comme waypoints, et le dernier noeud devient le terminus.
La ligne est ensuite envoyee au serveur via `setBusRoute`, puis elle peut etre supprimee via `deleteBusRoute`.

## Carte et rendu

`MapComponent` masque les vehicules en mode edition, resynchronise les selections apres une modification de carte et route les clics de noeuds vers le bon panneau.

`MapCanvas` garde le rendu de la carte, des intersections et des feux, puis transmet les clics selon l'outil actif.
`PixiApp` reste un composeur leger qui branche la carte sur Pixi.

La premiere phase de la branche a aussi ajoute une base de rendu dediee au type de vehicule, afin que les bus puissent etre traites comme une categorie distincte dans la simulation.