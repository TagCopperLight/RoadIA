<!-- Summary of test files in server/src/test -->

# Tests overview

Le dossier `server/src/test` contient des tests unitaires ciblant divers modules:

- `engine_tests.rs` — tests pour le moteur de simulation
- `kinematics_tests.rs` — vérifie fonctions physiques (arrival_time, v_stop_at, ...)
- `intersection_tests.rs` — tests de logique d'intersection (foe detection, link building)
- `editor_tests.rs` — tests des opérations d'édition de carte
- `simulation_tests.rs`, `vehicle_tests.rs` — tests hauts-niveau pour véhicules et scénarios

## Exécution

Exécuter tous les tests via `cargo test` depuis le répertoire `server`.

```bash
cd server
cargo test
```

## Recommandations

- Ajouter des tests reproductibles pour `compute_acceleration`, `rebuild_drive_plan` et scénarios d'intersection critique.
- Documenter comment ajouter de nouveaux tests et exécuter subsets via `cargo test --test <name>`.
