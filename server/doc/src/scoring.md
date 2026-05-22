# Scoring system

Fichier: `src/scoring/mod.rs` (et autres fichiers de scoring).

But:
- Calculer un score global sur la simulation basé sur: temps de trajet total, émissions de CO2, distance parcourue, taux de succès (arrivées).

Utilisation:
- `SimulationEngine::get_score()` appelle le module `scoring` pour produire un `Score` struct, qui est ensuite envoyé au client via `ServerPacket::Score`.
