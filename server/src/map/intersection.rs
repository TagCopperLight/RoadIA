use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intersection {
    pub id: u32,
    pub kind: IntersectionKind,
    pub name: String,
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntersectionKind {
    Habitation,
    Intersection,
    Workplace,
}
