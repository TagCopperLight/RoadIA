# Scoring system

Fichier: `src/scoring/mod.rs`.

Le module de scoring transforme l'état final d'une simulation en un nombre unique et lisible. Il compare les véhicules arrivés à des références théoriques pour le temps, le CO2 et la taille du réseau.

## Constantes physiques et de calibration

| Constante | Rôle |
|---|---|
| `AIR_DENSITY` | Densité de l'air utilisée pour l'effort aérodynamique. |
| `GRAVITY` | Accélération de la pesanteur. |
| `DRIVE_TRAIN_EFFICIENCY` | Rendement global de la transmission mécanique. |
| `FRANCE_GRID_CO2_PER_J` | Équivalent CO2 par joule électrique pour le mix français. |

## Structures internes et publiques

### `KindParams`

Paramètres physiques dépendant du type de véhicule.

- masse
- coefficient de traînée
- surface frontale
- résistance au roulement
- puissance consommée à l'arrêt

Ces paramètres permettent d'estimer de façon cohérente la consommation et les émissions.

### `MinHeap`

Petit wrapper de file de priorité utilisé lors du calcul de l'arbre couvrant minimal approché. Il sert uniquement à faire avancer l'algorithme de coût minimal sur les points de la carte.

### `Score`

Résultat complet envoyé au client à la fin de la simulation.

| Champ | Rôle |
|---|---|
| `score` | Valeur finale synthétique. |
| `total_trip_time` | Temps réellement consommé par les véhicules arrivés. |
| `ref_total_trip_time` | Temps de référence théorique minimal. |
| `total_emitted_co2` | CO2 réellement produit. |
| `ref_total_emitted_co2` | CO2 de référence minimal. |
| `network_length` | Longueur totale des routes distinctes. |
| `ref_network_length` | Borne inférieure du meilleur réseau possible. |
| `success_rate` | Part des véhicules arrivés. |

## Fonctions de calcul

### kind_params
Fournit les paramètres physiques pour une catégorie de véhicule donnée.
- **Entrées** : `kind: VehicleKind`.
- **Retour** : Structure `KindParams`.

### emission_params
Retourne les paramètres d'émission pour un couple (motorisation, catégorie).

- **Entrées** : `motorization: VehicleType`, `kind: VehicleKind`.
- **Retour** : `(f32, f32)` représentant (CO2 par Joule, CO2 par seconde au ralenti).

### update_co2_emissions
Calcule et ajoute le CO2 produit par un véhicule durant un pas de temps.

- **Entrées** :
    - `vehicle: &mut Vehicle` : Le véhicule à mettre à jour.
    - `time_step: f32` : Durée du pas de temps.
- **Action** : Calcule l'effort de traction (résistance air, roulement, accélération) puis la consommation énergétique et les émissions correspondantes.
- **Post-conditions** : `vehicle.emitted_co2` est incrémenté.

### get_minimal_time_travel_by_road / get_minimal_co2_by_road
Calculent les minima théoriques pour un véhicule sur une route donnée sans trafic ni contrainte.
- **Entrées** : `map`, `road_index`, `motorization`, `kind`.
- **Retour** : `f32`.

### get_vehicle_min_time / get_vehicle_min_co2
Estiment le temps et les émissions minimums pour l'intégralité du trajet d'un véhicule.
- **Action** : Somment les minima de chaque route composant le chemin du véhicule.
- **Retour** : `f32`.

### euclidean
Calcul de la distance Euclidienne en 64 bits.
- **Retour** : `f64`.

### mst_length
Calcule la longueur totale de l'arbre couvrant minimal pour un nuage de points (algorithme de Prim).
- **Retour** : `f64`.

### steiner_lower_bound
Estime la longueur minimale théorique nécessaire pour connecter tous les points d'intérêt (habitations et lieux de travail).

- **Entrées** : `map: &Map`.
- **Action** : Calcule l'Arbre Couvrant Minimal (MST) sur l'ensemble des points d'intérêt, puis applique un coefficient correctif de $\frac{\sqrt{3}}{2} \approx 0.866$ pour approcher la borne inférieure de Steiner.
- **Retour** : `f64` (longueur en mètres).

### compute_score
Calcule le score final synthétique de la simulation.

- **Entrées** :
    - `vehicles: &[Vehicle]` : Liste de tous les véhicules.
    - `config: &SimulationConfig` : Configuration incluant les poids du score.
- **Action** : 
    1. Agrège les temps de trajet et émissions réelles de tous les véhicules.
    2. Calcule les références théoriques (minima).
    3. Calcule le taux de réussite (véhicules arrivés / total).
    4. Combine les ratios (Ref/Réel) pondérés par les poids de la configuration.
- **Retour** : Structure `Score`.

## Lecture du score

Le score final compare toujours l'exécution réelle à une exécution de référence:

- si le trajet est rapide, le terme temps augmente;
- si beaucoup de véhicules arrivent, le terme succès augmente;
- si le CO2 réel est proche du CO2 minimal, le terme pollution augmente;
- si le réseau est compact, le terme infrastructure augmente.

Le résultat final est ensuite envoyé au client sous forme de `ServerPacket::Score`.

