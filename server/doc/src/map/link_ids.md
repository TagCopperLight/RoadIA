# Link IDs & how to obtain them

This page explains how `Link` and `InternalLane` IDs are produced and how to obtain them to configure traffic‑light phases or inspect movements.

Background
- `build_intersections(&mut map)` constructs `InternalLane` and `Link` objects for each junction and assigns unique `link.id` values. These `link.id` values are used by the engine (DrivePlanEntry.link_id) and by traffic light phases.

How to list link ids (Rust snippet)

```rust
// After the map has been fully constructed and build_intersections(map) called
for node_idx in map.graph.node_indices() {
    let node = &map.graph[node_idx];
    println!("Intersection {}: internal_lanes={}", node.id, node.internal_lanes.len());
    for il in &node.internal_lanes {
        println!("  InternalLane id={} from_lane={} to_lane={} length={}", il.id, il.from_lane_id, il.to_lane_id, il.length);
    }
}

for edge_idx in map.graph.edge_indices() {
    let road = &map.graph[edge_idx];
    println!("Road {}: lanes={}", road.id, road.lanes.len());
    for lane in &road.lanes {
        for link in &lane.links {
            println!("  link.id={} -> dest_road={} via_internal_lane={}", link.id, link.destination_road_id, link.via_internal_lane_id);
        }
    }
}
```

Practical workflow for traffic light phases
1. Build the intersections: call `build_intersections(&mut map)` after the map topography is final.
2. Run the listing code above (or log/print) to collect candidate `link.id` values for each desired green phase.
3. Call `add_traffic_light_controller(map, intersection_id, phases)` with `phases: Vec<(Vec<link_id>, green_duration, yellow_duration)>`.

Notes
- `link.id` values are globally unique across the map and stable while the map structure is unchanged. If you edit the map and re-run `build_intersections`, ids may change — regenerate the list after each structural edit.
- `serialize_map` does not expose link ids; you must inspect the server-side `Map` structure (via logging or a debug endpoint) to collect them.
