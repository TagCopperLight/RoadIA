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

La simulation par défaut utilise la carte en croix fournie par `map_generator`.
Le scénario schedulé actuel est hardcodé dans `SimulationInstance::new_default()` et utilise deux profils fixes.

Les détails du scheduling, du constructeur aléatoire Beta, des unités en secondes et du graphe des lois sont décrits dans [Scheduling](scheduling.md).

## Remarque

Cette documentation décrit l'état actuel du scheduler interne, sans génération Beta aléatoire ni configuration externe des profils.
Cette partie viendra dans une étape suivante.
