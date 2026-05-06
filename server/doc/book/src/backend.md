# Backend bus

## Type de vehicule

Le moteur distingue maintenant les voitures et les bus avec `VehicleKind`.
`VehicleSpec` embarque une `passenger_capacity`, ce qui permet de normaliser le CO2 par passager au lieu de penaliser les bus comme des voitures individuelles.

`BusSpecifications::default_spec()` fournit les parametres physiques du bus : vitesse max, acceleration, deceleration confortable, temps de reaction et longueur.

## Routage et etat des bus

Le modele `Vehicle` a ete etendu pour supporter un chemin plus riche que la simple destination finale :

- `waypoints` stocke les intersections intermediaires.
- `current_waypoint_index` indique le waypoint courant.
- `get_current_destination()` retourne soit le prochain waypoint, soit la destination finale.
- `advance_to_next_waypoint()` recalcule le chemin vers l'etape suivante.
- `clear_waypoints()` remet le vehicule a zero lors d'un reset.

## Simulation

`SimulationEngine` conserve un registre `bus_states` pour l'etat specifique des bus.
Chaque tick de simulation applique les etapes suivantes :

- mise a jour des temporisateurs d'arret des bus
- gestion des departs des vehicules
- traitement des arrivées des bus et passage au waypoint suivant si besoin
- calcul des plans de mouvement, des priorites et des feux
- mise a jour des emissions de CO2

Quand un bus arrive a une etape intermediaire, il repart vers la suivante au lieu de terminer definitivement son trajet.

## Websocket et routes

Le serveur expose un registre de routes de bus dans `SimulationInstance`.
Les packets `SetBusRoute` et `DeleteBusRoute` permettent de creer ou supprimer une ligne depuis l'interface.

Les updates vehicules envoyees au client transportent `kind`, `state` et `heading`, ce qui permet de distinguer les bus des voitures au rendu.

## Initialisation

La branche a d'abord pose un point d'entree capable de generer des vehicules de simulation a partir de la carte OSM.
Les commits suivants ont branche cette base sur des routes de bus definies depuis l'interface, avec un routage plus proche du cas d'usage final.