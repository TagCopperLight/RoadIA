<!-- Auto documentation: extracted from server/src/map/roundabout.rs -->

# `map::roundabout`

Fonctions utilitaires pour finaliser la construction d'un rond-point dans le `Map`.

## Rôle

- Ajuste les types de liens (yield) pour les entrées du ring et les forces opposées (foe_links).
- Utilisé après création d'une structure de ring (anneau) composée de `ring_node_ids` et `ring_road_ids`.

## API publique

- `pub struct RoundaboutHandle { ring_node_ids: Vec<u32>, ring_road_ids: Vec<u32> }`
  - `ring_node_ids`: identifiants des intersections formant l'anneau.
  - `ring_road_ids`: identifiants des routes (edges) du ring.

- `pub fn finalize_roundabout_links(map: &mut Map, handle: &RoundaboutHandle)`
  - Parcourt les noeuds du ring, identifie les liens entrants externes et marque ces `link.link_type = LinkType::Yield`.
  - Pour les routes du ring, recherche `foe_links` (liens opposés) qui pointent vers des entrées externes et marque `foe.link_type = LinkType::Yield`.

## Comportement détaillé

- Construction:
  - Crée un `ring_node_set` à partir de `ring_node_ids`.
  - Pour chaque `ring_node`, récupère les arêtes entrantes et identifie celles provenant de noeuds non-ring (entrées externes).
  - Tous les links de ces arêtes entrantes sont marqués `Yield`.
  - Ensuite, pour chaque route du ring, parcourt ses `lanes` → `links` → `foe_links` et, si un `foe` est dans `entry_link_ids`, marque aussi `foe.link_type = Yield`.

## Hypothèses & effets

- Suppose que `map.find_node` et `map.find_edge` trouvent les indices correspondants.
- Effet: modifie l'état interne des `Link` / `Lane` du `Map` pour intégrer la priorité du rond-point.

## Exemple (pseudocode)

```ignore
let handle = RoundaboutHandle { ring_node_ids: vec![10,11,12], ring_road_ids: vec![20,21,22] };
finalize_roundabout_links(&mut map, &handle);
// Maintenant les liens entrants externes ont LinkType::Yield
```

---

Souhaites-tu que j'ajoute un diagramme Mermaid expliquant le flux (entrées → mark yield → update foe_links) ?
