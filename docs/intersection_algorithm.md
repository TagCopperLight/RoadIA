# Intersection Management Algorithm

## 1. Data Model

### The Map (Graph)

The map is a directed petgraph `Graph<Intersection, Road>`. Intersections are not just nodes connecting edges: they contain their own internal lanes.

**Road**: a directed road segment between two intersections (a graph edge). A road contains one or more parallel **lanes**, numbered from the centre of the road: lane 0 is the leftmost lane, the highest index the rightmost one. Defined in `map/road.rs`.

**Lane**: a single drivable strip, with a list of **links** at its end pointing to where vehicles can go next.

**Intersection**: a graph node with a centre and a radius, holding the **internal lanes** that physically cross it. Defined in `map/intersection.rs`.

**InternalLane**: the lane a vehicle drives on while crossing an intersection, from one approach lane to one exit lane. Each link has exactly one internal lane.

**Link**: a connection from the end of a lane to a lane of the next road, through an internal lane (`via_internal_lane_id`). A link has a `link_type` (`Priority`, `Yield`, `Stop` or `TrafficLight`) and the list of its **foe links**.

```
Normal Lane A  →  [Link]  →  InternalLane (inside Intersection)  →  Normal Lane B
                      ↑
                  foe_links: [FoeLink from Lane C, FoeLink from Lane D, ...]
```

`build_intersections` creates a link from every incoming lane to every outgoing lane of every other road. U-turns are only created at dead ends, where they are the only way out.

Roundabouts imported from OpenStreetMap (`junction=roundabout`) and those created with the editor get `Yield` links on every road entering the ring (`map/roundabout.rs`).

---

## 2. The Conflict Graph (Foe Links)

Computed once when the network is built. Two links of the same intersection are foes when their internal lanes cross (`segments_intersect()`) or when they merge into the same lane. Links leaving the same lane are never foes.

The engine derives a few more static facts per link (`simulation/engine/network.rs`):

- its **rank**: `Stop` = 0, `Yield` = 1, `Priority` and `TrafficLight` = 2;
- its **turn** (straight, left, right, U-turn), from the directions of the roads;
- whether it must **keep the junction clear** (section 7).

---

## 3. Vehicle State

Each vehicle (`simulation/vehicle.rs`) has:

- `current_lane` (`LaneId::Normal(edge, lane)` or `LaneId::Internal(junction, internal_lane)`) and `position_on_lane` (position of its front);
- `velocity`;
- `drive_plan`: the junctions it is about to cross (section 5);
- `waiting_time`: how long it has been standing at its stop line, and `impatience`, which grows with it.

---

## 4. The Simulation Step

`step()` (`simulation/engine/mod.rs`) runs these phases, each for every vehicle before the next one starts:

1. **departures**: waiting vehicles enter their first road, on the lane with the most room, once there is room;
2. **signals**: traffic light phases advance (green, yellow, then the next phase);
3. **planning**: every vehicle on a road or inside a junction rebuilds its drive plan;
4. **lane changes** (LC2013 style: overtake on the left when blocked by a slow leader, keep right otherwise);
5. **registration**: drive plans are published on the links;
6. **movement**: every vehicle decides which links it may cross, follows its leader and moves.

Lanes are always processed in a fixed order (sorted lane ids, from the back of each lane), so a simulation is reproducible; `SimulationEngine::set_seed` also seeds the random navigation mistakes.

---

## 5. Planning and Registration

A vehicle looks ahead over `braking distance + 3 s of driving + 50 m` and lists every junction in that range (`simulation/engine/planning.rs`). For each one it estimates (`simulation/kinematics.rs`):

- `approach_speed`: the speed it slows down to before the stop line (road speed for `Priority` and `TrafficLight`, 70 % of it for `Yield`, 0 for `Stop`);
- `arrival_time` and `arrival_speed`: accelerate or brake towards the approach speed, then cruise. When the distance is too short to reach that speed, the reachable speed is used. A vehicle never plans to cruise below a creep speed, so a stopped vehicle still has a finite arrival time;
- `leave_time`: when its rear has left the internal lane, accelerating from the arrival speed;
- for `Stop` links, the remaining stop time is added to the arrival time.

If the route asks for a turn that does not exist, the vehicle is rerouted through the best existing exit of that junction.

Registration then walks every lane **from front to back**. This lets every vehicle know its leader:

- a follower never plans to cross the stop line less than `QUEUE_HEADWAY` after its leader;
- a vehicle waiting at its stop line gets a **queue rank** (how long it has waited). The vehicles queued behind it share that rank: they cannot go before it.

Each registration on a link holds the arrival and leave times, plus the queue rank for the vehicle's next junction only.

---

## 6. Right of Way (`simulation/engine/right_of_way.rs`)

`check_link()` tells whether a vehicle may cross a link, or why not (`Blocker`). The checks, in order:

1. **Signal**: red blocks unless the vehicle cannot stop even when braking hard; yellow blocks if the vehicle can stop comfortably.
2. **Stop sign**: the vehicle must have stood at the line for `STOP_DWELL_TIME`.
3. **Junction busy**: a vehicle on a foe internal lane that will still be there when the vehicle arrives.
4. **Exit full**: see section 7.
5. **Foes**: for every foe link the vehicle must yield to (below), every vehicle registered on it:
   - foes stuck behind a full exit are ignored: they cannot move;
   - when both vehicles are queued at their stop lines and the links have the same rank, the queue that waited longest goes first;
   - otherwise the vehicle is blocked if its time window overlaps the foe's, with a safety margin of `CONFLICT_TIME_GAP` (impatient drivers accept up to half of it).

Reckless vehicles ignore signals and stop signs.

### Who yields to whom

`LinkInfo::must_yield_to`:

- a lower rank yields to a higher rank;
- between equal ranks: traffic coming from the right has priority; facing traffic, the vehicle turning across the other's path (left or U-turn) yields; on parallel approaches, the left one yields;
- in remaining ties, the link with the higher id yields.

For every pair of foe links exactly one of them yields. Together with the queue ranks (a strict order between waiting vehicles), this makes it impossible for vehicles waiting at a junction to all yield to each other forever, which was the cause of most deadlocks in the previous version.

---

## 7. Keeping the Junction Clear

A vehicle stopped inside a junction blocks the flows crossing it. A link is marked `keep_clear` when it crosses a foe of the same or a higher rank going to another lane. On such links a vehicle only enters if the exit lane has room for it: the free space behind the last vehicle of that lane (plus a short extrapolation when that vehicle is moving), minus the space claimed by vehicles already inside the junction and heading there.

Links whose crossing foes all yield to them (a roundabout ring, for example) do not need this: vehicles move up as a queue, which lets a full ring move instead of locking.

A genuinely saturated network can still gridlock (a loop of full roads). This is deliberate: there is no teleporting, and the score reflects it.

---

## 8. Moving (`simulation/engine/movement.rs`)

For each vehicle:

1. **Decide**: check the links of the drive plan within `braking distance + 2 s + margin`, in order. Open links are counted; the first closed one becomes a virtual stopped obstacle at its stop line. Open links also cap the speed so that the vehicle can slow down to their approach speed.
2. **Follow**: find the leader, on the same lane or further along the route through the open links (internal lane, then exit lane). The vehicle that last left the lane into the junction is also followed while it is still inside it.
3. **Accelerate** with the IDM (`Vehicle::compute_acceleration`) towards the speed limit (or the vehicle's top speed, whichever is lower), against the closest of the leader and the stop line.
4. **Move**, and cross lane ends: into the internal lane of the next open link (claiming room on the exit lane), out of an internal lane onto the exit lane, or arrive at the end of the last road. A vehicle can cross several short lanes in one step. Lane lists are updated immediately, so a vehicle entering a junction is visible to the vehicles that move after it in the same step.
5. **Wait**: a vehicle standing within `minimum gap + STOP_LINE_TOLERANCE` of its stop line accumulates `waiting_time`.

---

## 9. Trips

- Arrived vehicles leave their lane and every link registration.
- A commute's return vehicle departs once the outbound vehicle arrived and the waiting time passed. Commutes with no way back have no return vehicle.
- Buses loop: at the end of their route they wait to depart again with their stops restored.
- With a small probability a vehicle takes a wrong exit and is rerouted; only exits leading to a drivable route are chosen.

---

## 10. Diagnostics

`examples/stress.rs` runs a scenario (a saved map or a generated grid) and reports arrivals, overlapping vehicles, step timings, and for stuck vehicles the chain of what each one is waiting for (`SimulationEngine::blocking_reason`):

```
cargo run --release --example stress -- grid:6 300 1 1800
cargo run --release --example stress -- map:data/<map>.json 500 1 3600
```
