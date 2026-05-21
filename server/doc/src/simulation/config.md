# Simulation configuration

Fichier: `src/simulation/config.rs`.

Ce chapitre décrit les paramètres qui contrôlent le comportement global du moteur de simulation, les poids du score et les constantes partagées entre les algorithmes de déplacement.

## Structures

### `ScoreWeights`

Pondérations appliquées au score final. Elles sont copiées depuis les réglages de la carte afin que le score reste cohérent avec la configuration de projet.

| Champ | Rôle |
|---|---|
| `time` | Favorise les trajets rapides. |
| `success` | Favorise les véhicules arrivés à destination. |
| `pollution` | Favorise une faible émission de CO2. |
| `infrastructure` | Favorise un réseau routier compact. |

La méthode `ScoreWeights::from_settings` lit ces coefficients directement dans les réglages de la carte (`MapSettings`).

### `SimulationConfig`

Configuration concrète passée au moteur.

| Champ | Rôle |
|---|---|
| `start_time` | Temps simulé de départ. |
| `end_time` | Temps simulé de fin. |
| `time_step` | Pas de temps de la boucle de simulation. |
| `minimum_gap` | Distance minimale entre deux véhicules consécutifs sur une voie. |
| `map` | Carte active utilisée par le moteur. |
| `score_weights` | Pondérations utilisées lors du calcul final du score. |

La méthode `SimulationConfig::new` construit une configuration prête à l'emploi en partant d'une carte et en appliquant les poids stockés dans `MapSettings`.

## Constantes de réglage

| Constante | Rôle |
|---|---|
| `MAX_SPEED` | Borne supérieure générale utilisée par les accélérations et la sélection de vitesse. |
| `ACCELERATION_EXPONENT` | Exposant utilisé dans la loi de type IDM pour lisser la montée en vitesse. |
| `LOOK_AHEAD` | Fenêtre de prudence temporelle utilisée pour anticiper les conflits d'intersection. |
| `STOP_DWELL_TIME` | Durée minimale d'attente avant de repartir après un arrêt complet. |
| `IMPATIENCE_RATE` | Vitesse de croissance de l'impatience d'un véhicule bloqué. |
| `MIN_CREEP_SPEED` | Vitesse de reptation appliquée dans les situations très lentes. |
| `LANE_WIDTH` | Largeur nominale d'une voie pour le rendu et l'offset latéral. |

## Lecture rapide

- `ScoreWeights` dit comment interpréter les critères du score.
- `SimulationConfig` dit quand la simulation commence et s'arrête, à quel rythme elle avance, et à quelle distance les véhicules doivent se tenir.
- Les constantes définissent des ordres de grandeur communs entre le moteur, le calcul d'accélération et la logique d'intersection.