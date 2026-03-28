use crate::map::model::Map;

/// Configuration générale de la simulation.
pub struct SimulationConfig {
    /// Temps de départ de la simulation (s).
    pub start_time: f32,

    /// Temps de fin de la simulation (s).
    pub end_time: f32,

    /// Pas de temps utilisé pour chaque `step` (s).
    pub time_step: f32,

    /// Écart minimum entre véhicules (m).
    pub minimum_gap: f32,

    /// Copie de la carte utilisée par la simulation.
    pub map: Map,
}

/// Vitesse maximale prise en compte pour l'estimation heuristique (m/s).
pub const MAX_SPEED: f32 = 42.0;

/// Exposant utilisé dans le modèle d'accélération (IDM).
pub const ACCELERATION_EXPONENT: f32 = 4.0;