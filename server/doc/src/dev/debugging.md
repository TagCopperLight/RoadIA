# Debugging & visualization guide

This short guide explains how to inspect engine and map runtime state to debug intersection behavior and vehicle flows.

1) Inspect `link_states`
- Purpose: shows `ApproachData` (arrival/leave windows) for each `link.id`.
- Recommendation: add a temporary debug API endpoint in `runner` that returns a JSON dump of `instance.engine.lock().await.link_states` serialized with `serde_json`.

Rust example (debug helper inside runner, conceptual):

```rust
// inside an API handler with access to SimulationInstance
let eng = instance.engine.lock().await;
let dump = serde_json::to_string(&eng.link_states).unwrap();
println!("LINK_STATES_DUMP: {}", dump);
// respond with JSON to caller
```

2) Dump `internal_lanes` and `foe_links`
- Useful to understand geometric conflicts. Print the node's `internal_lanes` and each lane's `foe_links` with associated `link.id`.

3) Vehicles by lane and drive plans
- Dump `vehicles_by_lane` and per-vehicle `drive_plan` to see planned maneuvers and ordering.

4) Visual debugging
- Export the map snapshot (`serialize_map`) plus the `vehicles` list (`serialize_vehicle`) at a point in time and use a lightweight viewer (the client `Map` React component works) to overlay internal lanes and highlight `link.id`s currently in `link_states`.

5) Quick checks to diagnose blocked movement
- If ego is blocked while `is_link_open` returns false:
  - Check `link_states[link_id]` for approaching foes whose windows overlap.
  - Check `internal_lanes` occupancy: internal lane in_* may be occupied (prevents entry).
  - Check `green_links`: traffic-light state may be blocking.

6) Logging recommendations
- Add structured logs when `is_link_open` makes decisions: print `ego` id, link id, `arrival_time/leave_time`, list of `foes` considered and final boolean decision.
