# `src/api/websocket.rs`

Overview
- Implements the server-side WebSocket protocol used by clients to control and observe simulations.

Types
- `ConnectParams`: connection query parameters (`uuid`, `token`) used at handshake.
- `ClientPacket`: messages sent by clients (simulation control and map-edit commands).
- `ServerPacket`: messages sent by server (map snapshot, vehicle updates, map edit responses, final score).

Key functions
- `ws_handler`: validates `uuid`/`token` and upgrades the HTTP connection to a WebSocket.
- `ws_loop`: main per-connection loop; sends initial map snapshot and multiplexes between incoming client messages and simulation broadcast updates.
- `process_incoming_msg` / `handle_client_packet`: parse `ClientPacket`s and perform actions (start/stop/reset simulation, edit map via `map::editor`).
- `process_broadcast_msg` / `broadcast_map_edit_success`: helper paths for sending `ServerPacket` updates to the connected socket and broadcasting map-edit results.
- `serialize_map` / `serialize_vehicle` / `serialize_traffic_lights`: convert internal map, vehicle and traffic-light state into JSON-friendly `serde_json::Value` structures used in `ServerPacket` payloads.

Notes
- The WebSocket API enforces that map-edit operations are only accepted when the simulation is stopped and broadcasts edit successes to all subscribers.
- Serialization helpers centralize the server-to-client payload format so the client and server can evolve independently.

(This page replaces raw auto-generated signatures with concise explanations.)
