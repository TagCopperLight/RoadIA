use crate::map::model::Coordinates;

#[derive(Debug, Clone)]
pub enum IntersectionKind {
    Habitation,
    Intersection,
    Workplace,
}

#[derive(Clone)]
pub struct Intersection {
    pub id: u32,
    pub kind: IntersectionKind,
    pub center_coordinates: Coordinates,
}

impl Intersection {
    pub fn new(
        id: u32,
        kind: IntersectionKind,
        center_coordinates: Coordinates,
    ) -> Self {
        Self {
            id,
            kind,
            center_coordinates,
        }
    }
}