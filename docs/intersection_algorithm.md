# Intersection Management Algorithm

## 1. Data Model

### The Map (Graph)

The map is a directed petgraph `Graph<Intersection, Road>`. The key distinction from a naive road graph is that intersections are **not** just nodes connecting edges — they contain their own internal road segments.

**Road** — a directed road segment between two intersections (stored as a graph edge). A road contains one or more parallel **lanes**. Lanes are where vehicles actually drive; the road is just a grouping. Defined in `map/road.rs`.

**Lane** — a single drivable strip. Has a `length`, a `speed_limit`, and a list of **links** at its end pointing to where vehicles can go next.

**Intersection** — a node in the graph. Has a polygonal area (not just a point) defined by a `center_coordinates` and `radius`. Contains a set of **internal lanes** that physically represent the road space inside the intersection. Defined in `map/intersection.rs`.

**InternalLane** — a lane that lives inside an intersection. It connects one approach lane (`from_lane_id`) to one exit lane (`to_lane_id`). It is a full lane (has `length`, geometry via `entry`/`exit` points, `speed_limit`) but is invisible to the vehicle's route — the route only names normal roads; internal lanes are traversed transparently as part of crossing an intersection.

**Link** — a connection object that sits at the end of a lane and points to the next lane (`lane_origin_id` → `lane_destination_id`). If the destination is across an intersection, the link also holds a `via_internal_lane_id` pointing to the internal lane the vehicle will use to cross. A link has a `link_type` (`Priority`, `Yield`, `Stop`, or `TrafficLight`) and a precomputed list of **foe links**.

```
Normal Lane A  →  [Link]  →  InternalLane (inside Intersection)  →  [Link]  →  Normal Lane B
                      ↑
                  foe_links: [FoeLink from Lane C, FoeLink from Lane D, ...]
```

---

## 2. The Conflict Graph (Foe Links)

This is computed **once at network load time**, not during simulation.

For every link in the network, find all other links in the same intersection whose path geometrically crosses it. Those are its **foe links** (`Vec<FoeLink>`). Two paths cross if the internal lanes they use share any overlapping area, detected via `segments_intersect()`.

Each link stores:
- `foe_links: Vec<FoeLink>` — other connections that conflict with it geometrically
- `foe_internal_lane_ids: Vec<u32>` — the internal lanes those foe links use (needed to detect vehicles already inside the intersection)

This conflict graph is static. It never changes during simulation unless the network topology changes.

---

## 3. Vehicle State

Each vehicle (`Vehicle` in `simulation/vehicle.rs`) maintains:

- `current_lane: Option<LaneId>` and `position_on_lane: f32` — its current lane (either `LaneId::Normal(EdgeIndex, lane_id)` or `LaneId::Internal(intersection_id, internal_lane_id)`) and position on that lane
- `velocity: f32` — its current speed
- `drive_plan: Vec<DrivePlanEntry>` — an ordered list of upcoming links it intends to cross, each annotated with estimated arrival time, arrival speed, and leave speed
- `waiting_time: f32` — how long it has been stopped at the intersection boundary
- `impatience: f32` — grows with waiting time (affects gap acceptance, described later)

---

## 4. The Simulation Loop

Each simulation step (`step()` in `simulation/engine.rs`) has sequential phases that run globally across all vehicles before the next phase begins. The order is strict: every vehicle finishes planning before any vehicle registers, and every vehicle registers before any vehicle moves.

The full step sequence is:

1. `handle_departures()` — dequeue vehicles waiting to enter the simulation
2. `plan_movements()` — rebuild drive plans (Phase 1 below)
3. `attempt_lane_changes()` — evaluate the LC2013 lane-change model
4. `register_approaches()` — push drive plans onto links (Phase 2 below)
5. `advance_traffic_lights()` — update signal phases
6. `execute_movements()` — integrate physics and move vehicles (Phase 3 below)
7. `flush_transfers()` — commit pending lane transitions

---

### Phase 1 — Plan (`plan_movements`)

Every vehicle, on every active lane, runs its planning routine. The vehicle looks ahead along its planned route — far enough to cover its maximum braking distance — and builds a `drive_plan`: an ordered list of every link it expects to cross within that horizon.

For each link in the list, the vehicle computes and stores a `DrivePlanEntry` with:
- `v_pass: f32` — the speed at which it intends to cross the link if allowed through
- `v_wait: f32` — the speed it would need to brake to in order to stop at the link if blocked
- `arrival_time: f32` — kinematic estimate of when the vehicle's front will reach the link (see section 5)
- `arrival_speed: f32` — estimated speed at the moment of arrival
- `leave_time: f32` — estimated time when the vehicle's rear clears the link
- `leave_speed: f32` — estimated speed when the vehicle's rear clears the link
- `distance: f32` — current distance to the link
- `set_request: bool` — whether the vehicle actually intends to pass this link (false if, for example, the vehicle is stopping before it)

**The lane order during planning matters.** Vehicles are planned back-to-front on each lane: the vehicle at the rear of the lane plans first, using its leader's current speed as a constraint. The car-following model (IDM, in `simulation/kinematics.rs`) returns a safe speed that prevents the follower from rear-ending the leader. This safe speed feeds into the `v_pass` values in the drive plan.

---

### Phase 2 — Register (`register_approaches`)

Every vehicle now pushes its drive plan onto the links.

For each link in its `drive_plan`, the vehicle writes an entry into the link's **`LinkState`** (in `map/intersection.rs`). `LinkState` holds `approaching: HashMap<u64, ApproachData>` keyed by vehicle id, where `ApproachData` contains:
- `arrival_time: f32`
- `leave_time: f32` — computed as `arrival_time + (internal_lane_length + vehicle_length) / avg_speed`
- `arrival_speed: f32`
- `leave_speed: f32`
- `will_pass: bool` — true if the vehicle intends to cross, false if it is only approaching to stop

Before writing, the vehicle first removes its previous entries from all links in `registered_link_ids`. This ensures stale entries (from route changes or passed links) do not appear as ghost vehicles in foe checks.

After removal, the new entries are written. The link's `LinkState` now holds an up-to-date picture of who is coming and when.

**One important detail for all-way stops**: at the moment of registration, a small random offset is added to `arrival_time`. This breaks ties between vehicles that arrive at an all-way stop at exactly the same simulated time, preventing symmetric deadlocks.

---

### Phase 3 — Execute (`execute_movements`)

Every vehicle integrates forward. This phase has three sub-steps that happen in sequence within each vehicle:

#### Sub-step A — Determine next speed

The vehicle walks through its `drive_plan` front-to-back and determines the maximum safe speed for this tick. For each link in the plan:

1. **Yellow light** (`TrafficLight` in transition): if the signal is yellow and the vehicle has enough distance to stop, the safe speed is capped at `v_wait` and processing stops — the vehicle commits to braking.

2. **Passability check** (`is_link_open()`): the vehicle asks the link whether it can pass, providing its own arrival time, leave time, speeds, `impatience`, and deceleration as arguments. This check runs the full decision tree described in section 6: red light → continuation → priority → stop sign → gap acceptance against every foe.

3. **If the link is open**: the safe speed is set to `v_pass`. If the link is `Yield` and the vehicle is still far enough away to stop, it additionally checks whether foes are actually visible yet — if not, it still brakes to `v_wait` as a precaution until it can actually see oncoming traffic.

4. **If the link is closed**: the safe speed is capped at `v_wait`. The vehicle will decelerate to a stop at the link boundary.

5. **Point of no return**: if the vehicle is so close to a `Yield` link that it can no longer stop even with maximum deceleration, it proceeds regardless of the outcome of the passability check. This models real-world committed-entry behaviour. A minimum speed is enforced to ensure the vehicle clears the intersection quickly rather than dithering.

The output of this sub-step is a single scalar: the maximum safe speed the vehicle is allowed to reach this tick.

#### Sub-step B — Integrate position

The vehicle applies the speed determined above (filtered through the IDM car-following model for noise and minimum-speed clamping) and advances its position:

```
position_on_lane += velocity * dt
velocity          = chosen_speed
```

This always runs every simulation step.

#### Sub-step C — Handle lane transitions

After the new position is computed, the vehicle checks whether it has crossed the end of its current lane. It may have crossed more than one lane boundary in a single step if the lanes are very short (which internal lanes often are).

The vehicle loops while `position_on_lane` exceeds the length of the current lane:

1. Take the next `DrivePlanEntry` from the `drive_plan`.
2. Resolve the next lane: if the entry has a `via_internal_lane_id`, use `LaneId::Internal(junction_id, internal_lane_id)`; otherwise use the link's `lane_destination_id` as `LaneId::Normal(...)`.
3. **Hard red check**: if the link's signal is red and the vehicle is not past the stop line, this is an emergency stop — the vehicle halts.
4. Fire departure notifications on the current lane (move reminders, detector updates, etc.), then remove the vehicle from the lane's vehicle list.
5. Rebase `position_on_lane`: subtract the length of the lane just left, so the position is now measured from the start of the new lane.
6. Set the vehicle's `current_lane` to the new `LaneId`, fire arrival notifications, update route progress (only when entering a non-internal road).
7. Record intersection entry and exit timestamps if the link is the entry or exit of the intersection area.
8. **Repeat**: if `position_on_lane` still exceeds the length of the new lane, continue the loop with the next link. This handles fast vehicles crossing multiple short internal lanes in one tick.

---

### After all vehicles have executed: Buffer Flush (`flush_transfers`)

When a vehicle transitions into a new lane, it is not immediately inserted into that lane's sorted vehicle list. Instead, it is placed into a pending buffer on the target lane. Only after all vehicles on all lanes have finished executing does `flush_transfers` commit these buffers: each vehicle is removed from its old lane's list and inserted (in sorted order) into its new lane's list.

This two-stage approach avoids modifying the vehicle lists mid-iteration, which would corrupt the back-to-front traversal order and produce incorrect car-following results.

---

## 5. Arrival Time Estimation

This is the kinematic core used in Phase 1 (`simulation/kinematics.rs`).

Given:
- `dist` — distance from the vehicle's current position to the link
- `v0` — current speed (`velocity`)
- `v1` — estimated speed at the link (depends on link type; may require braking)
- `a_max` — vehicle's `max_acceleration`
- `d_max` — vehicle's `comfortable_deceleration` (positive value)

The minimum time to travel `dist` while transitioning from `v0` to `v1` is:

If `v1 >= v0` (speeding up or holding speed):
- Use `a = a_max`
- Compute the time and distance of the acceleration phase: `t_accel = (v1 - v0) / a`, `d_accel = t_accel * (v0 + v1) / 2`
- If `dist >= d_accel`: accelerate fully, then cruise the remaining distance at `max(v0, v1)`
  - `t_total = t_accel + (dist - d_accel) / max(v0, v1)`
- If `dist < d_accel`: the vehicle doesn't have room to reach `v1` — solve `d = v0*t + 0.5*a*t²` for `t`
  - `t_total = (-v0 + sqrt(v0² + 2*a*dist)) / a`

If `v1 < v0` (slowing down):
- Use `a = -d_max`, same logic with deceleration instead of acceleration

The **leave time** is then the arrival time plus the time to clear the internal lane:

```
leave_time   = arrival_time + (internal_lane_length + vehicle_length) / average_speed
average_speed = (arrival_speed + leave_speed) / 2
```

The estimated `arrival_speed` at the link itself is determined before calling this:
- For a `Priority` link: `v1` = the vehicle's natural cruising speed
- For a `Yield` link (must yield): `v1` may be reduced to a slow creep speed so the vehicle approaches cautiously and leaves room to stop
- For a red `TrafficLight`: `v1 = 0`

---

## 6. Passability Check (Right-of-Way)

When a vehicle wants to cross a link, it calls `is_link_open()`. This returns either **open** (the vehicle may proceed) or **blocked** (it must wait).

The check is a decision tree evaluated in order. The first matching condition wins.

### 6.1 Red signal

If the link has a `TrafficLight` type and the current signal phase is red, the link is blocked unconditionally.

### 6.2 Already inside the intersection

If the vehicle is already partway through the intersection on an `InternalLane` (a continuation link, identified by `LaneId::Internal`), it is always allowed to continue. Stopping mid-intersection would cause gridlock.

### 6.3 Priority link

If the link has `link_type = Priority` (main road, major green phase), the link is open. Foe vehicles do not block priority vehicles.

### 6.4 Stop sign

If the link has `link_type = Stop`, the vehicle must wait a minimum dwell time (`STOP_DWELL_TIME = 1.0` second in `simulation/config.rs`) before being allowed to proceed, even if no foes are present. This models the real-world stop rule.

### 6.5 Gap acceptance

For `Yield` links and minor green phases, the vehicle checks every `FoeLink` in its link's `foe_links` list and determines whether any approaching foe vehicle would conflict with it.

For each foe link, the vehicle iterates the foe link's `LinkState.approaching` map and evaluates each `ApproachData` entry using the **time-window check** described in section 7 (`time_window_conflict()`). If any foe vehicle blocks the ego, the link is blocked.

---

## 7. Time-Window Conflict Check (`time_window_conflict`)

This is the core of gap acceptance. Given:

- Ego's time window: `[arrival_time, leave_time]`
- Foe's time window: `[foe_arrival_time, foe_leave_time]`
- A safety margin `LOOK_AHEAD = 0.1` seconds (one simulation step, from `simulation/config.rs`)
- Whether ego and foe share the same target lane (a merge scenario)

Three scenarios:

**Scenario A — Ego would follow the foe** (`foe_leave_time < arrival_time`)

The foe clears the conflict zone before ego arrives. Ego could follow. Blocked if:
- The gap is too small: `arrival_time - foe_leave_time < LOOK_AHEAD`
- Or the merge is physically unsafe: the foe's post-junction speed and ego's approach speed are such that ego cannot avoid rear-ending the foe even if both brake at their maximums

**Scenario B — Ego would lead the foe** (`leave_time + LOOK_AHEAD < foe_arrival_time`)

Ego clears the conflict zone before the foe arrives. Ego could lead. Blocked if:
- The foe cannot safely decelerate behind ego: ego's exit speed vs. foe's approach speed means the foe's braking distance is shorter than ego's
- Right-of-way override: `foe_is_to_the_right()` gives priority to a foe coming from the right

**Scenario C — Time windows overlap**

Ego and foe would be in the conflict zone at the same time. Always blocked.

The `LOOK_AHEAD` margin ensures a small safety gap between the two time windows even in the follower/leader cases.

---

## 8. Impatience and Foe Braking Assumption

Vehicles accumulate `impatience` (in `[0, 1]`) as they wait at an intersection via `waiting_time`.

When `impatience` is nonzero and ego would arrive at the conflict zone before the foe (i.e., ego would be leader), RoadIA assumes the foe *may* yield by braking. The foe's effective arrival time is adjusted:

```
adjusted_foe_arrival = lerp(real_foe_arrival_time, foe_arrival_if_braking, impatience)
```

Where `foe_arrival_if_braking` is the time the foe would arrive if it began braking at the moment ego reaches the conflict zone. At `impatience = 0`, the foe is treated as unyielding. At `impatience = 1`, the ego assumes the foe will fully yield.

This models the real-world behaviour where a sufficiently impatient driver forces their way through a gap, and oncoming traffic yields.

---

## 9. All-Way Stop Precedence

All-way stops use a different priority mechanism — not gap acceptance, but waiting time ordering:

1. The vehicle with the highest `waiting_time` goes first.
2. If two vehicles have waited the same time, the one with the earlier `arrival_time` goes first.
3. A small random tie-breaker is injected at registration time (a 0 or 1 step offset on `arrival_time`) to prevent exact ties from causing deadlock.

---

## 10. Vehicles Already Inside the Intersection

A vehicle that has already entered the intersection and is on an `InternalLane` (`LaneId::Internal(...)`) presents a different problem: it is not in any link's `LinkState.approaching` map (it has already passed the link), but it is physically occupying the conflict zone.

This is handled via the `foe_internal_lane_ids` list. When a vehicle runs `is_link_open()`, it also checks the occupancy of each foe internal lane directly (not just the `approaching` map of the foe link). Any vehicle already driving on a foe internal lane is treated as blocking, regardless of its registered arrival time.

---

## 11. Action Steps vs. Simulation Steps

Replanning (Phase 1 and Phase 2) does not have to happen every simulation step. Each vehicle's `VehicleSpec.reaction_time` determines how frequently it replans. On steps that are not action steps:

- The vehicle's position is still integrated normally (physics always runs every step at `DEFAULT_TIME_STEP_S = 0.1` seconds)
- The `drive_plan` is not rebuilt
- The registered arrival times on the links are not updated — other vehicles see stale data

This is a performance trade-off. The arrival time is an estimate of a future event, and safety margins in the gap acceptance (`LOOK_AHEAD`) are sized to absorb the resulting error.

---

## 12. Summary: What Each Entity Does Per Step

**Each vehicle (per action step):**
1. Look ahead along the route for all links within the braking horizon
2. For each link, estimate `arrival_time` and `arrival_speed` using kinematics
3. Remove old `ApproachData` from links in `registered_link_ids`
4. Push new `ApproachData` onto all upcoming links' `LinkState.approaching` maps
5. Run `is_link_open()` on the immediate next link
6. If the link is open: proceed, transition to the `InternalLane`, then to the exit lane
7. If blocked: hold position, accumulate `waiting_time` and `impatience`

**Each intersection (static, no per-step logic):**
- Holds the conflict graph (`foe_links` on each `Link`, `foe_internal_lane_ids`)
- Holds the `Vec<InternalLane>`
- Does not run any decision logic itself — all decisions are made by the vehicles

**Each link (passive data store via `LinkState`):**
- Holds its `link_type` (`Priority`, `Yield`, `Stop`, `TrafficLight`)
- Holds `approaching: HashMap<u64, ApproachData>` of registered vehicles and their time windows
- Exposes `is_link_open()` as a query function

The key architectural point is that **intersections do not orchestrate vehicles** — every vehicle independently decides whether it can go, by querying the shared state (the `LinkState.approaching` maps) that all vehicles have written. The intersection is just a geometric and topological container.
