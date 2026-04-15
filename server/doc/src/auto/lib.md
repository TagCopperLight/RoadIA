<!-- Auto documentation: extracted from server/src/lib.rs -->

# `lib` (crate root)

Point d'agrégation du crate `server` : expose les modules `api`, `map`, `simulation`, `scoring`.

## Rôle

- Fournit la surface publique du crate pour être utilisé par le binaire ou des tests.

## Remarques

- Ne contient pas de logique métier lourde; sert à organiser et ré-exporter les modules.
# `src/lib.rs`

Overview
- Crate root re-exporting the primary modules: `api`, `map`, `scoring` and `simulation`. The `test` module is compiled only for test builds.

Notes
- The heavy lifting lives in the submodules; this file acts as the crate entry point for library consumers and the server binary.
