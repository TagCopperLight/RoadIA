# Kinematics helpers

Fichier: `src/simulation/kinematics.rs`.

Les fonctions de ce module servent à convertir une distance restante, une vitesse courante et des bornes d'accélération en temps estimé ou en vitesse d'arrêt possible. Elles sont utilisées par le moteur pour réserver les intersections et construire les `DrivePlanEntry`.

Unités: distances en mètres, vitesses en mètres par seconde, durées en secondes.

## `v_stop_at(dist, d_max)`

Cette fonction calcule la vitesse maximale qui permettrait de s'arrêter exactement sur une distance `dist` avec une décélération confortable `d_max`. La relation utilisée est celle du mouvement uniformément décéléré: $v = \sqrt{2 d_{max} \cdot dist}$.

Exemple d'ordre de grandeur: sur 25 m avec une décélération de 3 m/s², la vitesse d'arrêt théorique est d'environ 12,25 m/s.

## `arrival_time(dist, v0, v1, a_max, d_max)`

Cette fonction estime le temps minimal nécessaire pour parcourir `dist` en partant de `v0` et en rejoignant `v1`, tout en respectant une accélération maximale `a_max` et une décélération maximale `d_max`.

Le moteur s'en sert pour savoir quand un véhicule atteindra une jonction et quand il la quittera. Lorsque la distance est courte, la fonction tient compte du fait qu'il n'y a pas toujours une phase de croisière: tout peut être consacré à accélérer ou à freiner.

## `leave_time(t_arrive, lane_len, veh_len, v_arrive, v_leave)`

Cette fonction transforme une heure d'arrivée en heure de sortie estimée, en considérant la longueur utile de la voie, la longueur du véhicule et une vitesse moyenne bornée. Elle produit une fenêtre temporelle plus réaliste qu'un simple instant ponctuel, ce qui évite de faire se chevaucher artificiellement plusieurs véhicules dans une même intersection.

## `approach_speed(link_type, road_speed_limit)`

Cette fonction choisit la vitesse d'approche cible d'un lien selon sa priorité:

- un lien prioritaire conserve la vitesse de la route,
- un lien en cédez-le-passage est volontairement plus lent,
- un arrêt impose une vitesse nulle,
- un feu tricolore se comporte comme une route normale tant qu'il est vert.

## Ce qu'il faut retenir

- `v_stop_at` répond à la question: "à quelle vitesse dois-je être pour pouvoir m'arrêter à temps ?"
- `arrival_time` répond à la question: "quand vais-je arriver au carrefour ?"
- `leave_time` répond à la question: "quand aurai-je complètement dégagé le carrefour ?"
- `approach_speed` répond à la question: "à quelle vitesse dois-je aborder ce lien ?"
