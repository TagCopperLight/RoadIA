# Roundabouts

Fichier principal: `src/map/roundabout.rs`.

Ce chapitre couvre les helpers dédiés à la finalisation des ronds-points après leur création géométrique.

## Structures

### `RoundaboutHandle`

Handle retourné par la création d'un rond-point.

| Champ | Rôle |
|---|---|
| `ring_node_ids` | Identifiants des noeuds formant l'anneau. |
| `ring_road_ids` | Identifiants des routes de l'anneau. |

## Fonction principale

### `finalize_roundabout_links`

Cette fonction adapte les priorités après la création du rond-point.

Elle identifie les liens d'entrée venant de l'extérieur du cercle et les transforme en mouvements de type `Yield`. Elle parcourt ensuite les routes de l'anneau pour marquer comme `Yield` les `foe_links` en conflit avec ces entrées, ce qui permet de modéliser correctement la priorité d'insertion dans le rond-point.

## Lecture fonctionnelle

- le handle conserve les identifiants créés par l'éditeur;
- `finalize_roundabout_links` applique les règles de priorité après la géométrie brute;
- le module n'ajoute pas les noeuds lui-même, il complète un rond-point déjà construit.