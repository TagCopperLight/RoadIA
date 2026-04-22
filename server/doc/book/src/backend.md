# Backend

Le backend RoadIA expose un serveur Axum sur `8080` avec deux points d'entrée principaux:

- `POST /api/simulations` pour créer la simulation par défaut basée sur la carte en croix et des départs différés.
- `WS /ws` pour piloter la simulation et recevoir les mises à jour.

## Durée de simulation

Les simulations sont bornées par `MAX_DURATION = 86400` secondes, soit une journée.
Le nombre maximal de steps est calculé à partir de cette durée et du `time_step` de la simulation.

## Départ des véhicules

Le moteur de simulation prend désormais en compte `TripRequest.departure_time`.
Un véhicule en attente ne quitte pas sa position tant que le temps courant est inférieur à sa date de départ.

## Simulation par défaut

La simulation par défaut utilise désormais la carte en croix fournie par `map_generator`.
Le scheduler actuel est volontairement simple et déterministe.
Il accepte une liste de profils de shifts avec:

- un point de départ `origin`
- un point d'arrivée `destination`
- une `departure_time`
- une `dwell_time`

Chaque profil crée un véhicule aller au démarrage.
Quand ce véhicule arrive à destination, le scheduler crée le véhicule retour avec:

- `origin` et `destination` inversés
- une date de départ égale à `arrived_at + dwell_time`

Les paramètres sont validés côté serveur: les temps doivent être positifs, les nœuds doivent exister dans la carte et un profil en double est refusé.

## Remarque

Cette documentation décrit l'état actuel du scheduler, sans génération Beta aléatoire.
Cette partie viendra dans une étape suivante.
