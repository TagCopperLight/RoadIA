# Scoring system

Fichier: `src/scoring/mod.rs` (et autres fonctions utilitaires).

But:
- Calculer un score global sur la simulation basé sur: temps de trajet total, émissions de CO2, distance parcourue, taux de succès (arrivées).

Utilisation:
- `SimulationEngine::get_score()` appelle le module `scoring` pour produire un `Score` struct, qui est ensuite envoyé au client via `ServerPacket::Score`.

## Détails de l'implémentation

Le scoring combine plusieurs termes normalisés : temps, succès (ratio véhicules arrivés), pollution et infrastructure. Les poids actuels (dans `scoring/mod.rs`) sont :

- `TIME_WEIGHT = 0.4`
- `SUCCESS_WEIGHT = 0.2`
- `POLLUTION_WEIGHT = 0.2`
- `INFRASTRUCTURE_WEIGHT = 0.2`

Le calcul suit ces étapes :

1. `success_rate` = véhicules arrivés / total
2. `total_trip_time` = maximum du temps de trajet observé (arrived_at - departure_time)
3. `sum_trip_time` = somme des temps effectifs pour véhicules arrivés
4. `total_ref_trip_time` = somme des temps théoriques minimaux (`get_vehicle_min_time`) sur les mêmes trajets
5. `time_term` = `total_ref_trip_time / sum_trip_time` (proche de 1 si efficaces)
6. `pollution_term` = `total_ref_emitted_co2 / total_emitted_co2`
7. `infrastructure_term` = `best_network_length (Steiner lower bound) / network_length`

Score final :

```
score = TIME_WEIGHT * time_term
	+ SUCCESS_WEIGHT * success_rate
	+ POLLUTION_WEIGHT * pollution_term
	+ INFRASTRUCTURE_WEIGHT * infrastructure_term
```

## Unités et constantes

- Les fonctions de co2 utilisent des constantes physiques/empiriques (masse, rendement moteur, surface frontale, densité de l'air, etc.) pour estimer les émissions par distance parcourue.

## Exemple d'interprétation

- `score` dans `[0,1]` (théoriquement) : plus élevé = meilleur compromis temps/pollution/succès/infrastructure.

