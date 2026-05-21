# Roads & Lanes

Fichier: `src/map/road.rs`.

Ce module contient les structures qui décrivent une route, ses voies et les mouvements possibles à travers les intersections.

## `LinkType`

Type sémantique d'un mouvement à travers une intersection.

- `Yield` : cédez le passage.
- `Priority` : mouvement prioritaire.
- `Stop` : arrêt obligatoire.
- `TrafficLight` : mouvement contrôlé par un feu.

## `Road`

Section orientée du graphe entre deux intersections.

| Champ | Rôle |
|---|---|
| `id` | Identifiant unique de la route. |
| `length` | Longueur utile de la section. |
| `speed_limit` | Limite de vitesse de la route. |
| `lane_width` | Largeur nominale d'une voie. |
| `lanes` | Voies appartenant à cette route. |

### `Road::new`

Construit une route avec plusieurs voies.

- **Entrées** :
    - `id` : Identifiant public de la route.
    - `lane_count` : Nombre de voies à créer.
    - `speed_limit` : Vitesse limite (m/s).
    - `length` : Longueur de la route (m).
- **Action** : Initialise la structure `Road` et crée le nombre demandé d'objets `Lane`. La vitesse est bornée par `MAX_SPEED`.
- **Retour** : Une instance de `Road`.

## `Lane`

Voie de circulation appartenant à une route.

| Champ | Rôle |
|---|---|
| `id` | Identifiant local de la voie. |
| `road_id` | Identifiant de la route parente. |
| `length` | Longueur de la voie. |
| `speed_limit` | Limite de vitesse copiée depuis la route. |
| `links` | Mouvements autorisés depuis cette voie. |

## `Link`

Mouvement logique reliant une voie entrante à une route sortante via une intersection.

| Champ | Rôle |
|---|---|
| `id` | Identifiant global du mouvement. |
| `lane_origin_id` | Identifiant local de la voie entrante. |
| `lane_destination_id` | Identifiant local de la voie de sortie. |
| `via_internal_lane_id` | Identifiant de la voie interne traversée. |
| `destination_road_id` | Route de sortie visée. |
| `link_type` | Règle de priorité appliquée. |
| `entry` | Point géométrique d'entrée dans le carrefour. |
| `junction_center` | Centre géométrique du carrefour. |
| `foe_links` | Mouvements concurrents pouvant bloquer ce lien. |
| `foe_internal_lane_ids` | Identifiants des voies internes en conflit direct. |

## `FoeLink`

Mouvement adverse associé à un lien en conflit.

| Champ | Rôle |
|---|---|
| `id` | Identifiant du lien adverse. |
| `link_type` | Type de priorité de ce lien adverse. |
| `entry` | Point d'entrée géométrique du mouvement adverse. |

## Lecture fonctionnelle

- `Road` porte la structure de haut niveau.
- `Lane` porte l'état de circulation local sur la route.
- `Link` porte les mouvements possibles dans les intersections.
- `FoeLink` porte la notion de conflit.

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
