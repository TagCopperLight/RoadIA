use crate::simulation::config::{ACCELERATION_EXPONENT, MAX_SPEED};
use petgraph::graph::{EdgeIndex, NodeIndex};

use crate::map::{model::Coordinates, model::Map};

/// Type de véhicule supporté par la simulation.
#[derive(Copy, Clone)]
pub enum VehicleKind {
    /// Véhicule particulier (voiture).
    Car,
    /// Transport en commun, par ex. bus.
    Bus,
}

/// Spécification physique et comportementale d'un véhicule.
#[derive(Copy, Clone)]
pub struct VehicleSpec {
    /// Type du véhicule (`Car`, `Bus`, ...).
    pub kind: VehicleKind,

    /// Vitesse maximale souhaitée (m/s).
    pub max_speed: f32,

    /// Accélération maximale disponible (m/s^2).
    pub max_acceleration: f32,

    /// Décélération confortable (m/s^2) utilisée pour les calculs d'IDM.
    pub comfortable_deceleration: f32,

    /// Temps de réaction du conducteur (s).
    pub reaction_time: f32,

    /// Longueur du véhicule (m).
    pub length: f32,
}

/// Requête de trajet pour un véhicule: origine, destination et horaire.
#[derive(Clone)]
pub struct TripRequest {
    /// Nœud d'origine dans le graphe de la carte.
    pub origin: NodeIndex,

    /// Nœud de destination dans le graphe de la carte.
    pub destination: NodeIndex,

    /// Temps de départ (ex: timestamp en secondes).
    pub departure_time: u64,

    /// Optionnel: temps de retour si applicable.
    pub return_time: Option<u64>,
}

/// État d'un véhicule dans la simulation.
#[derive(Copy, Clone, PartialEq)]
pub enum VehicleState {
    /// En attente de départ.
    WaitingToDepart,
    /// Actuellement sur une route.
    OnRoad,
    /// Arrivé à destination.
    Arrived,
}

/// Représentation d'un véhicule dans la simulation avec son état et son itinéraire.
#[derive(Clone)]
pub struct Vehicle {
    /// Identifiant unique.
    pub id: u64,

    /// Spécification du véhicule.
    pub spec: VehicleSpec,

    /// Requête de trajet associée au véhicule.
    pub trip: TripRequest,

    /// État courant du véhicule.
    pub state: VehicleState,

    /// Trajet calculé sous forme de liste de nœuds.
    pub path: Vec<NodeIndex>,

    /// Index courant dans le chemin (`path`).
    pub path_index: usize,

    /// Position actuelle sur la route (distance entre l'avant du véhicule et le début de la route).
    pub position_on_road: f32,

    /// Position au pas de temps précédent.
    pub previous_position: f32,

    /// Vitesse actuelle (m/s).
    pub velocity: f32,

    /// Vitesse au pas de temps précédent.
    pub previous_velocity: f32,
}

/// Calcule le chemin le plus rapide entre `source` et `destination` en utilisant A*.
///
/// La métrique utilise la longueur des routes et les limites de vitesse pour estimer
/// le coût temporel.
pub fn fastest_path(map: &Map, source: NodeIndex, destination: NodeIndex) -> Vec<NodeIndex> {
    let result = petgraph::algo::astar(
        &map.graph,
        source,
        |finish| finish == destination,
        |e| e.weight().length / f32::from(e.weight().speed_limit),
        |n| map.intersections_euclidean_distance(n, destination) / f32::from(MAX_SPEED),
    );
    match result {
        Some((_cost, path)) => path,
        None => Vec::new(),
    }
}

impl Vehicle {
    /// Construit un nouveau véhicule avec état initial et chemin vide.
    pub fn new(id: u64, spec: VehicleSpec, trip: TripRequest) -> Self {
        Self {
            id,
            spec,
            trip,
            state: VehicleState::WaitingToDepart,
            path: Vec::new(),
            path_index: 0,
            previous_velocity: 0.0,
            velocity: 0.0,
            position_on_road: 0.0,
            previous_position: 0.0,
        }
    }

    /// Met à jour le chemin du véhicule vers sa destination en recalculant le plus rapide.
    pub fn update_path(&mut self, map: &Map) {
        self.path = fastest_path(map, self.trip.origin, self.trip.destination);
        self.path_index = 0;

        if self.path.len() < 2 {
            self.state = VehicleState::Arrived;
        }
    }

    /// Calcule l'accélération souhaitée selon l'IDM (Intelligent Driver Model).
    ///
    /// `desired_velocity` : vitesse souhaitée (m/s),
    /// `minimum_gap` : écart minimum de sécurité (m),
    /// `vehicle_ahead_distance` : distance jusqu'au véhicule précédent (m),
    /// `vehicle_ahead_velocity` : vitesse du véhicule précédent (m/s).
    pub fn compute_acceleration(
        &self,
        desired_velocity: f32,
        minimum_gap: f32,
        vehicle_ahead_distance: f32,
        vehicle_ahead_velocity: f32,
    ) -> f32 {
        let free_road_acc = self.spec.max_acceleration
            * (1.0 - (self.previous_velocity / desired_velocity).powf(ACCELERATION_EXPONENT));

        if vehicle_ahead_distance <= 0.0 {
            panic!("Vehicle ahead is too close");
        }
        let s: f32 = minimum_gap
            + self.previous_velocity * self.spec.reaction_time
                + 0.5 * self.previous_velocity * (self.previous_velocity - vehicle_ahead_velocity)
                    / (self.spec.max_acceleration * self.spec.comfortable_deceleration)
                        .powf(0.5);

        free_road_acc - self.spec.max_acceleration * (s / vehicle_ahead_distance).powf(2.0)
    }

    /// Retourne les coordonnées 2D du véhicule sur la carte en interpolant
    /// sa position le long de la route courante.
    pub fn get_coordinates(&self, map: &Map) -> Coordinates {
        let current_node = map
            .graph
            .node_weight(self.get_current_node())
            .ok_or("Vehicle not in map")
            .unwrap();
        match self.state {
            VehicleState::OnRoad => {
                let next_node_o = map
                    .graph
                    .node_weight(self.get_next_node())
                    .ok_or("Vehicle not in map")
                    .unwrap();
                let current_road = map
                    .graph
                    .edge_weight(
                        map.graph
                            .find_edge(self.get_current_node(), self.get_next_node())
                            .ok_or("Edge not in map")
                            .unwrap(),
                    )
                    .ok_or("Edge not in map")
                    .unwrap();

                let pos_rate: f32 = self.position_on_road / current_road.length;
                Coordinates {
                    x: current_node.x * (1.0 - pos_rate) + next_node_o.x * pos_rate,
                    y: current_node.y * (1.0 - pos_rate) + next_node_o.y * pos_rate,
                }
            }
            _ => Coordinates {
                x: current_node.x,
                y: current_node.y,
            },
        }
    }

    /// Renvoie le nœud courant du chemin (`path[path_index]`).
    pub fn get_current_node(&self) -> NodeIndex {
        self.path[self.path_index]
    }

    /// Renvoie le nœud suivant dans le chemin. Panique si le véhicule est arrivé.
    pub fn get_next_node(&self) -> NodeIndex {
        if self.path_index + 1 >= self.path.len() {
            panic!("Vehicle is at destination");
        }
        self.path[self.path_index + 1]
    }

    /// Renvoie l'index de l'arête (route) entre le nœud courant et le nœud suivant.
    pub fn get_current_road(&self, map: &Map) -> EdgeIndex {
        map.graph
            .find_edge(self.get_current_node(), self.get_next_node())
            .ok_or("Edge not in map")
            .unwrap()
    }
}
