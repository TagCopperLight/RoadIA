# Kinematics helpers

Fichier: `src/simulation/kinematics.rs`.

Ce chapitre donne des exemples numériques pour les fonctions utilitaires: `arrival_time`, `leave_time`, `v_stop_at`, `approach_speed`.

Unités: distances en mètres (m), vitesses en m/s, durées en secondes (s).

## `v_stop_at(dist, d_max)`

- Formule: $v = \sqrt{2\,d_{max}\,dist}$

Exemple:

- distance = 25 m, d_max = 3.0 m/s²

```text
v = sqrt(2 * 3.0 * 25) = sqrt(150) ≈ 12.247 m/s
```

Interprétation: une vitesse jusqu'à ~12.25 m/s peut être arrêtée sur 25 m avec décélération 3.0 m/s².

## `arrival_time(dist, v0, v1, a_max, d_max)`

- Calcule le temps minimal pour parcourir `dist` en partant à `v0` et en arrivant à `v1` sous contraintes d'accélération/décélération.

Exemple numérique simple (cas accélération):

- dist = 100 m, v0 = 10 m/s, v1 = 20 m/s, a_max = 2.0 m/s², d_max = 2.0 m/s²

Calcul conceptuel (résumé):

1. Estimer le temps nécessaire pour accélérer de 10→20 m/s avec a_max = 2.0: t_acc = (v1 - v0)/a_max = 5.0 s.
2. Distance parcourue pendant l'accélération approximative: s_acc = (v0 + v1)/2 * t_acc = 15 * 5 = 75 m.
3. Reste à parcourir en croisière: s_cruise = 100 - 75 = 25 m.
4. Temps de croisière: t_cruise = s_cruise / v1 = 25 / 20 = 1.25 s.
5. Total approximé: t_total ≈ t_acc + t_cruise = 6.25 s.

> Note: l'implémentation exacte de `arrival_time` tient compte des limites, des phases et renvoie une valeur proche de ce calcul.

## `leave_time(t_arrive, lane_len, veh_len, v_arrive, v_leave)`

- Règle: estimer le temps de sortie en supposant une vitesse moyenne bornée ≥ 0.1 m/s.

Exemple:

- t_arrive = 12.0 s, lane_len = 8.0 m, veh_len = 4.0 m, v_arrive = 5.0 m/s, v_leave = 13.9 m/s

1. avg_speed ≈ max( (5.0 + 13.9)/2, 0.1 ) = 9.45 m/s
2. t_leave = 12.0 + (lane_len + veh_len) / avg_speed = 12.0 + 12.0 / 9.45 ≈ 13.27 s

## `approach_speed(link_type, road_speed_limit)`

- Heuristiques:
  - `Priority` → `road_speed_limit`
  - `Yield` → `0.7 * road_speed_limit`
  - `Stop` → `0.0`
  - `TrafficLight` → `road_speed_limit`

Exemple: `link_type = Yield`, `road_speed_limit = 13.9` → approach_speed ≈ 0.7 * 13.9 = 9.73 m/s

---

Ces exemples aident à comprendre les ordres de grandeur et tester la cohérence des `DrivePlanEntry` produits par le moteur.
