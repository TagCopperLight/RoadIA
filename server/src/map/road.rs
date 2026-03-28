use crate::simulation::config::MAX_SPEED;

/// Représente une route (arête) entre deux intersections.
#[derive(Clone)]
pub struct Road {
    /// Identifiant unique de la route.
    pub id: u32,

    /// Nombre de voies.
    pub lane_count: u8,

    /// Limite de vitesse (m/s).
    pub speed_limit: f32,

    /// Longueur de la route (m).
    pub length: f32,

    /// Indique si la route est momentanément bloquée.
    pub is_blocked: bool,

    /// Indique si le dépassement est autorisé.
    pub can_overtake: bool,
}

impl Road {
    /// Construit une nouvelle route en clampant la `speed_limit` entre 1 et `MAX_SPEED`.
    pub fn new(
        id: u32,
        lane_count: u8,
        speed_limit: f32,
        length: f32,
        is_blocked: bool,
        can_overtake: bool,
    ) -> Self {
        Self {
            id,
            lane_count,
            speed_limit: speed_limit.clamp(1.0, MAX_SPEED),
            length,
            is_blocked,
            can_overtake,
        }
    }
}
