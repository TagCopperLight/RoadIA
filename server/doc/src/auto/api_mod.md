<!-- Auto documentation: extracted from server/src/api/mod.rs -->

# `api` (module)

Point d'entrée des routes HTTP / WebSocket exposées par le serveur.

## Sous-modules

- `websocket` — gestion des paquets WS (`ClientPacket` / `ServerPacket`), handler d'upgrade et boucle WS.
- `runner` — code d'orchestration serveur (création d'instances de simulation, routes `/ws` et `/api/simulations`).

## Intégration

- `api::runner::run()` est appelé par le binaire pour démarrer le serveur axum.
# `src/api/mod.rs`

Overview
- Module index that re-exports API submodules used by the server executable.

Submodules
- `runner`: server entrypoint, HTTP routes and simulation instance lifecycle management.
- `websocket`: WebSocket protocol implementation and serialization helpers for client-server messaging.

Notes
- Keep this module small; main logic lives in each submodule to keep responsibilities clear.
