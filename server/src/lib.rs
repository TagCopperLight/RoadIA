//! Bibliothèque principale du serveur RoadIA.
//!
//! Expose les modules `api`, `map` et `simulation` pour construire et exécuter
//! la simulation côté serveur.
pub mod api;
pub mod map;
pub mod simulation;

#[cfg(test)]
pub mod test;
