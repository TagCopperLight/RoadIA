<!-- Auto documentation: extracted from server/src/map/mod.rs -->

# `map` (module)

Regroupe la logique de carte: `model`, `intersection`, `road`, `editor`, `osm_parser`, `roundabout`, `traffic_light`.

## Rôle

- Fournit la représentation `Map` (graphe), outils d'édition et parseur OSM pour importer des régions.

## Remarques

- Les utilitaires d'édition (`map::editor`) mettent le `Map` dans un état cohérent attendu par le planner et l'engine.
