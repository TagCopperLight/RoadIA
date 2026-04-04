# Roads & Lanes

Fichier: `src/map/road.rs`.

Types exposés:
- `Road` : représente une arête du graphe, contient `id`, `length`, `speed_limit`, `lane_width`, `lanes: Vec<Lane>`.
- `Lane` : parcourt une route, a `id`, `road_id`, `length`, `speed_limit`, `links: Vec<Link>`.
- `Link`, `FoeLink` et `LinkType` : décrivent les connexions entre lanes, les priorités et les entrées pour les intersections.

Constructeur:
- `Road::new(id, lane_count, speed_limit, length)` crée les `lane_count` lanes.

Rôle:
- Les `links` d'une `Lane` définissent comment les véhicules traversent les intersections (liaisons internes, priorités, etc.).
