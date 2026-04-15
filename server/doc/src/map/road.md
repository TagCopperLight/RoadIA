# Roads & Lanes

Fichier: `src/map/road.rs`.

Types exposés et champs (champ-par-champ)

- `Road`:
	- `id: u32` — identifiant unique.
	- `length: f32` — longueur utile de la section entre intersections (m).
	- `speed_limit: f32` — limite de vitesse pour la route (m/s).
	- `lane_width: f32` — largeur des voies (m).
	- `lanes: Vec<Lane>` — tableaux de `Lane` attachées à cette route.

- `Lane`:
	- `id: u32` — identifiant local de la voie.
	- `road_id: u32` — id de la `Road` parente.
	- `length: f32` — généralement égal à `road.length`.
	- `speed_limit: f32` — copie locale de la limite pour access rapide.
	- `links: Vec<Link>` — mouvements sortants définissant les destinations possibles depuis cette voie.

- `Link`:
	- `id: u32` — identifiant du mouvement (utilisé dans `DrivePlanEntry` et `link_states`).
	- `destination_road_id: u32` — id de la `Road` de sortie ciblée par ce mouvement.
	- `via_internal_lane_id: u32` — id de l'`InternalLane` traversée dans l'intersection.
	- `link_type: LinkType` — sémantique du mouvement (`Priority`, `Yield`, `Stop`, `TrafficLight`).
	- `foe_links: Vec<FoeLink>` — liste des `FoeLink` (liens en conflit) utilisée par `is_link_open`.

- `FoeLink`:
	- `id: u32` — id du lien adverse en conflit.
	- `link_type: LinkType` — type du lien adverse (utile si feux/priority changent).
	- `angle` / heuristiques (implémentation) — orientation relative utilisée pour priorité à droite.

Constructeurs et helpers

- `Road::new(id, lane_count, speed_limit, length)` — crée la `Road` et initialise `lane_count` voies avec `lane_width` par défaut.

Rôle et utilisation

- Les `links` d'une `Lane` correspondent aux mouvements autorisés à la traversée d'une intersection. Ils sont référencés par `DrivePlanEntry.link_id` lors de la planification et servent de clef pour `link_states` (enregistrement des approches). `foe_links` représente conflits physiques (croisements / merges).

JSON / payload (format côté API)

Exemple `Road` JSON (tel que sérialisé par `serialize_map`):

```json
{
	"id": 2,
	"from": 1,
	"to": 3,
	"lane_count": 2,
	"lane_width": 3.0,
	"length": 120.0,
	"speed_limit": 13.9
}
```

Exemple conceptuel d'un `Link` (non directement sérialisé entier par le serveur, mais utilisé par l'engine):

```json
{
	"id": 101,
	"destination_road_id": 3,
	"via_internal_lane_id": 55,
	"link_type": "Yield",
	"foe_links": [ { "id": 201, "link_type": "Priority" } ]
}
```

Notes:
- Les `link.id` sont uniques et globalement utilisés pour l'enregistrement d'approches et la synchronisation entre clients et engine.
- Les structures internes (Lane, Link, FoeLink) servent principalement à la logique de planification; l'API n'expose que un sous-ensemble via `serialize_map`.
