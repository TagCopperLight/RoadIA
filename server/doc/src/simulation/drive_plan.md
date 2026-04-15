# DrivePlanEntry & LinkState (JSON examples)

This page shows the runtime structures produced by the engine and how they look conceptually as JSON. These are not directly serialized by the server as a public API, but they describe the internal data the engine uses.

## DrivePlanEntry (fields)

- `link_id` (u32): id of the internal movement (see `map/link_ids.md`).
- `lane_id` (LaneId): origin lane (Normal(edge, lane_idx) or Internal(junction, il_id)).
- `via_internal_lane_id` (u32): internal lane id traversed at the junction.
- `junction_id` (u32): intersection id.
- `v_pass` (f32): pass velocity (m/s).
- `v_wait` (f32): wait velocity if blocked (m/s).
- `arrival_time`, `leave_time` (f32): estimated times in simulation seconds.
- `arrival_speed`, `leave_speed` (f32): speeds used for timing.
- `distance` (f32): cumulative distance from current vehicle position to link.
- `set_request` (bool): whether the vehicle registers an approach request for this link.

Example DrivePlanEntry JSON (single entry):

```json
{
  "link_id": 101,
  "lane_id": { "Normal": [ 42, 0 ] },
  "via_internal_lane_id": 55,
  "junction_id": 10,
  "v_pass": 9.5,
  "v_wait": 3.0,
  "arrival_time": 12.34,
  "leave_time": 12.90,
  "arrival_speed": 8.5,
  "leave_speed": 7.0,
  "distance": 45.2,
  "set_request": true
}
```

## LinkState and ApproachData

`LinkState` holds approaching vehicles keyed by vehicle id. `ApproachData` contains estimated arrival/leave times and speeds.

Example `LinkState` JSON:

```json
{
  "link_id": 101,
  "approaching": {
    "42": { "arrival_time": 12.34, "leave_time": 12.9, "arrival_speed": 8.5, "leave_speed": 7.0, "will_pass": true },
    "1001": { "arrival_time": 13.5, "leave_time": 14.1, "arrival_speed": 9.0, "leave_speed": 8.0, "will_pass": false }
  }
}
```

How the engine uses these structures
- After `rebuild_drive_plan`, vehicles with `set_request=true` insert `ApproachData` into `link_states[link_id].approaching`.
- `is_link_open` consults `link_states` and `foe_links` to decide whether the `ego` vehicle may proceed.

Debug tip: to reproduce engine decisions, record the `DrivePlanEntry` list for the ego vehicle and the `LinkState.approaching` map for all `foe_links`, then evaluate `time_window_conflict` on arrival/leave intervals.
