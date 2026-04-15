<!-- Auto documentation: extracted from server/src/map/road.rs -->

# `map::road`

Types de données décrivant routes, voies et liens entre voies (links) utilisés
par le planner et l'engine.

## Types et champs

- `LinkType` : enum `{ Yield, Priority, Stop, TrafficLight }` — priorité du mouvement.

- `Road` : {
  - `id: u32`
  - `length: f32` (m)
  - `speed_limit: f32` (m/s)
  - `lane_width: f32` (m)
  - `lanes: Vec<Lane>`
}

- `Lane` : {
  - `id: u32` (index local)
  - `road_id: u32`
  - `length: f32`
  - `speed_limit: f32`
  - `links: Vec<Link>` — mouvements sortants vers d'autres routes / internal lanes
}

- `Link` : mouvement atomique entre une lane et une destination
  - `id, lane_origin_id, lane_destination_id, via_internal_lane_id, destination_road_id`
  - `link_type` : priorité
  - `entry`, `junction_center` : coordonnées pour calcul géométrique
  - `foe_links`: vecteur de `FoeLink` (liens en conflit)

- `FoeLink` : `{ id, link_type, entry }` — représentation résumée d'un lien adverse.

## Constructeurs

- `Road::new(id, lane_count, speed_limit, length) -> Road` crée `lane_count` instances `Lane` et initialise `lane_width` à `LANE_WIDTH` (config).

## Remarques

- `speed_limit` et `lane.speed_limit` sont clampés entre `1.0` et `MAX_SPEED`.
- Les `Link` sont créés par `map::intersection::build_intersection` en fonction
  des combinaisons `incoming × outgoing` et reçoivent des `foe_links` pour gérer conflits.

## Exemple (concept)

```ignore
let road = Road::new(10, 2, 13.9, 120.0); // id=10, 2 voies, 13.9 m/s, 120 m
```

---
# `src/map/road.rs`

Overview
- Road-level types model physical road segments, their lanes and the directed links that connect lanes through intersections. These types capture geometry, speed limits and link semantics used by planning and execution logic.

Key types
- `LinkType`: movement semantics (Yield, Priority, Stop, TrafficLight) which influence right-of-way and approach behavior.
- `Road` / `Lane`: road aggregates lanes and lane metadata (length, speed limits). Lanes contain `Link` entries representing allowed movements to downstream lanes.
- `Link`: represents a permitted lane-to-lane movement, including possible internal intersection lane ids and references to conflicting (`foe`) links used for conflict resolution.

Notes
- Constructing roads via `Road::new` sets up per-lane data consistently; link/foe bookkeeping is used by `is_link_open` and drive-plan logic to check movement permissions at junctions.

Function parameters
- `Road::new(id: u32, lane_count: u8, speed_limit: f32, length: f32) -> Self`: create a `Road` with `lane_count` lanes, applying `speed_limit` and given length.
- `Lane` / `Link` types: fields are populated during intersection building; `Link` contains `link_type` and `entry`/`exit` geometry used by the planner.
