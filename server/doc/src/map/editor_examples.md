# Map editor examples

Cette page donne des repères d'utilisation pour le module d'édition sans recopier le code source.

## Ajouter un rond-point

Une création de rond-point typique repose sur un centre, un rayon et un nombre de bras suffisant pour laisser de la place aux branches. Le handle retourné permet ensuite de connecter les routes d'accès extérieures à chaque noeud de l'anneau.

## Ajouter un contrôleur de feux

Un contrôleur est décrit par une liste de phases. Chaque phase liste les liens autorisés et les durées de vert et de jaune. Les liens utilisés doivent être valides et être récupérés après reconstruction des intersections.

## Édition via le WebSocket

Les opérations de base disponibles côté client sont l'ajout, la suppression et la mise à jour de noeuds et de routes. Les helpers de plus haut niveau, comme la création de ronds-points ou de feux, restent des opérations serveur ou de démarrage de carte.

## Bon réflexe

- construire la topologie;
- reconstruire les intersections;
- récupérer les identifiants de liens;
- seulement ensuite configurer les feux ou les ronds-points avancés.
