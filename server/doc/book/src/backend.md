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
Le scénario schedulé est défini en dur dans `SimulationInstance::new_default()`.
À l'heure actuelle, les paramètres du scheduling ne sont pas fournis par le front: ils sont hardcodés dans le backend pour garder un comportement déterministe.

Le scénario actuel contient deux shifts:

- `1 -> 2` avec une `departure_time` à `5.0` secondes et un `dwell_time` de `5.0` secondes
- `3 -> 4` avec une `departure_time` à `10.0` secondes et un `dwell_time` de `2.0` secondes

Le scheduler interne crée d'abord les véhicules aller au démarrage. Quand un véhicule arrive à destination, le scheduler crée le véhicule retour avec:

- `origin` et `destination` inversés
- une date de départ égale à `arrived_at + dwell_time`

Le scheduler interne valide toujours les paramètres lorsqu'on l'instancie: les temps doivent être positifs, les nœuds doivent exister dans la carte et un profil en double est refusé. Cette validation reste utile pour la suite, même si les valeurs sont actuellement fixées côté serveur.

## Remarque

Cette documentation décrit l'état actuel du scheduler interne, sans génération Beta aléatoire ni configuration externe des profils.
Cette partie viendra dans une étape suivante.
