//! Gestion des intersections — implémentation simplifiée inspirée du modèle SUMO.
//!
//! NOTE (branch status): Cette implémentation couvre un sous-ensemble des
//! fonctionnalités décrites dans le document "Intersection Management Algorithm".
//! Ci-dessous un résumé rapide des éléments présents dans cette branche et de
//! ceux qui ne le sont pas (ou sont partiellement pris en charge).
//!
//! Implémenté dans cette branche
//! - Représentation d'une `Intersection` contenant des règles par route
//!   (`IntersectionRules`) et une file d'attente `traffic_order`.
//! - `IntersectionRequest` : stockage d'une requête d'accès avec angles
//!   d'entrée/sortie et `arrival_time`.
//! - Calcul géométrique de conflits (`paths_conflict` / `get_path_mask`) basé
//!   sur les angles d'entrée/sortie.
//! - Enregistrement des requêtes via `request_intersection`, détection des
//!   collisions entre requêtes et calcul d'un ordre d'accès (`insert_by_arrival_time`
//!   / `reorder_conflicting_group`).
//! - Vérification basique d'autorisation via `get_permission_to_enter` qui
//!   retourne `true` si le véhicule est en tête de `traffic_order`.
//! - Logique de priorité locale (`IntersectionController::determine_priority`) qui
//!   applique priorités et règle de priorité à droite.
//!
//! Non implémenté ou partiellement implémenté (différences avec SUMO)
//! - Pas de modèle de `Lane` / `Internal lane` ni de `Link` objet — la couche
//!   fine de voies internes n'existe pas.
//! - Pas de graphe statique de "foe links" pré-calculé au chargement du réseau.
//!   Les conflits sont détectés dynamiquement entre requêtes en comparant
//!   géométriquement les trajectoires (angles).
//! - Pas de table d'approche par lien contenant des fenêtres [arrival, leave]
//!   pour chaque véhicule. Seuls les `IntersectionRequest` (avec `arrival_time`)
//!   sont stockées et utilisées pour l'ordonnancement.
//! - Pas de système de feux/états de lien détaillés (rouge/jaune/vert) ni
//!   d'autorité centrale de feu; la notion `TrafficLight` existe mais n'a pas
//!   de gestion temporelle dans le contrôleur.
//! - Pas de notion d'`impatience` ni d'ajustement des arrivals via
//!   gap-acceptance dynamique (seulement priorité et ordre par arrival_time).
//! - Pas d'occupation de "foe lanes" (vérification explicite des véhicules
//!   déjà dans la zone d'intersection) — la présence physique n'est pas
//!   suivie au niveau interne des voies.
//!
//! Conséquence architecturale : le système de décision est limité — les
//! véhicules écrivent des requêtes d'arrivée et l'intersection calcule un
//! ordre statique/locale. La prise de décision avancée (fenêtres temporelles,
//! gap acceptance, random tie-breaking sur all-way stop, etc.) n'est pas
//! disponible dans cette branche.
//!
use std::collections::HashMap;
use std::cmp::Ordering::{Equal, Greater, Less};

/// Catégorie sémantique d'une intersection / nœud de la carte.
#[derive(Debug, Clone)]
pub enum IntersectionKind {
    /// Zone résidentielle.
    Habitation,
    /// Simple croisement/intersection.
    Intersection,
    /// Zone de travail / destination professionnelle.
    Workplace,
}

/// Règles associées à une entrée d'intersection.
#[derive(Clone, Debug, PartialEq)]
pub enum IntersectionRules {
    /// Céder le passage.
    Yield,
    /// Route prioritaire.
    Priority,
    /// Stop.
    Stop,
    /// Feu de circulation.
    TrafficLight,
}

/// Type physique / signalisation d'une intersection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IntersectionType {
    /// Intersection avec priorité.
    Priority,
    /// Intersection avec stop.
    Stop,
    /// Intersection contrôlée par feu.
    TrafficLight,
}

/// Représente une intersection du graphe, avec son contrôleur et ses requêtes.
#[derive(Clone)]
pub struct Intersection {
    /// Identifiant unique de l'intersection.
    pub id: u32,

    /// Catégorie sémantique (habitation, workplace, ...).
    pub kind: IntersectionKind,

    /// Nom lisible de l'intersection.
    pub name: String,

    /// Coordonnées x,y.
    pub x: f32,
    pub y: f32,

    /// Type de signalisation de l'intersection.
    pub intersection_type: IntersectionType,

    /// Règles par route entrante (road_id -> rule).
    pub rules: HashMap<u32, IntersectionRules>,

    /// Requêtes courantes d'accès à l'intersection.
    pub requests: Vec<IntersectionRequest>,

    /// Ordre d'accès (file/ordre) calculé pour les véhicules.
    pub traffic_order: Vec<u64>,
}

/// Représente une demande d'accès à une intersection par un véhicule.
#[derive(Clone)]
pub struct IntersectionRequest {
    /// Identifiant du véhicule demandeur.
    pub vehicle_id: u64,

    /// Règle applicable pour cette entrée.
    pub rule: IntersectionRules,

    /// Angle d'entrée (degré) vers l'intersection.
    pub entry_angle: f32,

    /// Angle de sortie (degré) depuis l'intersection.
    pub exit_angle: f32,

    /// Temps d'arrivée estimé (s).
    pub arrival_time: f32,
}

/// Petits utilitaires pour déterminer l'ordre de priorité dans les conflits.
pub struct IntersectionController;

impl Intersection {
    pub fn new(
        id: u32,
        kind: IntersectionKind,
        name: String,
        x: f32,
        y: f32,
        intersection_type: IntersectionType,
    ) -> Self {
        Self {
            id,
            kind,
            name,
            x,
            y,
            intersection_type,
            rules: HashMap::new(),
            requests: Vec::new(),
            traffic_order: Vec::new(),
        }
    }

    pub fn set_rule(&mut self, road_id: u32, rule: IntersectionRules) {
        self.rules.insert(road_id, rule);
    }

    pub fn get_rule(&self, road_id: u32) -> IntersectionRules {
        match self.rules.get(&road_id) {
            Some(rule) => rule.clone(),
            None => panic!("Road {} not found in intersection {}", road_id, self.id),
        }
    }

    pub fn get_permission_to_enter(&self, vehicle_id: u64) -> bool {
        self.traffic_order.first() == Some(&vehicle_id)
    }

    pub fn request_intersection(
        &mut self,
        vehicle_id: u64,
        rule: IntersectionRules,
        arrival_time: f32,
        from: (f32, f32),
        to: (f32, f32),
    ) {
        let (entry_angle, exit_angle) = self.compute_entry_exit_angles(from, to);
        let new_request = IntersectionRequest { vehicle_id, rule, entry_angle, exit_angle, arrival_time };

        let collisions = new_request.collisions_with(&self.requests, self.rules.len());
        self.requests.push(new_request.clone());

        if collisions.is_empty() {
            self.insert_by_arrival_time(vehicle_id, arrival_time);
        } else {
            self.reorder_conflicting_group(collisions, new_request);
        }
    }

    pub fn remove_request(&mut self, vehicle_id: u64) {
        self.requests.retain(|r| r.vehicle_id != vehicle_id);
        self.traffic_order.retain(|v| *v != vehicle_id);
    }

    fn compute_entry_exit_angles(&self, from: (f32, f32), to: (f32, f32)) -> (f32, f32) {
        let entry_angle = {
            let dx = self.x - from.0;
            let dy = self.y - from.1;
            dy.atan2(dx).to_degrees()
        };
        let exit_angle = {
            let dx = to.0 - self.x;
            let dy = to.1 - self.y;
            dy.atan2(dx).to_degrees()
        };
        (entry_angle, exit_angle)
    }

    fn get_path_mask(entry: f32, exit: f32, n: usize) -> u64 {
        let sector_width = 360.0 / n as f32;

        let entry_norm = (entry % 360.0 + 360.0) % 360.0;
        let exit_norm = (exit % 360.0 + 360.0) % 360.0;

        let in_angle = (entry_norm + 180.0) % 360.0;
        let in_sector = ((in_angle + sector_width / 2.0) / sector_width).floor() as usize % n;
        let out_sector = ((exit_norm + sector_width / 2.0) / sector_width).floor() as usize % n;

        let mut mask: u64 = 0;
        mask |= 1 << in_sector;
        mask |= 1 << out_sector;

        let diff = (out_sector + n - in_sector) % n;
        if diff != 1 {
            mask |= 1 << n; // Center bit
        }

        mask
    }

    fn paths_conflict(
        entry_angle_1: f32, exit_angle_1: f32, arrival_time_1: f32,
        entry_angle_2: f32, exit_angle_2: f32, arrival_time_2: f32,
        roads_count: usize,
    ) -> bool {
        const CROSSING_DURATION: f32 = 2.5;

        if (arrival_time_1 - arrival_time_2).abs() >= CROSSING_DURATION {
            return false;
        }
        if roads_count < 3 {
            return false;
        }

        let mask1 = Self::get_path_mask(entry_angle_1, exit_angle_1, roads_count);
        let mask2 = Self::get_path_mask(entry_angle_2, exit_angle_2, roads_count);

        (mask1 & mask2) != 0
    }

    fn insert_by_arrival_time(&mut self, vehicle_id: u64, arrival_time: f32) {
        let insert_index = self.traffic_order
            .iter()
            .position(|&id| {
                self.requests
                    .iter()
                    .find(|r| r.vehicle_id == id)
                    .map_or(false, |r| r.arrival_time > arrival_time)
            })
            .unwrap_or(self.traffic_order.len());
        
        let insert_index = if self.traffic_order.is_empty() { insert_index } else { insert_index.max(1) };
        self.traffic_order.insert(insert_index, vehicle_id);
    }

    fn reorder_conflicting_group(
        &mut self,
        collisions: Vec<IntersectionRequest>,
        new_request: IntersectionRequest,
    ) {
        let all_conflicting: Vec<IntersectionRequest> = collisions.iter().cloned()
            .chain(std::iter::once(new_request.clone()))
            .collect();
        let priority_order = IntersectionController::determine_priority(&all_conflicting);

        let new_rank = priority_order
            .iter()
            .position(|r| r.vehicle_id == new_request.vehicle_id)
            .unwrap_or(priority_order.len());

        let insert_idx = self.traffic_order
            .iter()
            .enumerate()
            .filter_map(|(pos, &tid)| {
                let rank = priority_order.iter().position(|r| r.vehicle_id == tid)?;
                if rank < new_rank { Some(pos + 1) } else { None }
            })
            .max()
            .unwrap_or_else(|| {
                self.traffic_order
                    .iter()
                    .enumerate()
                    .find_map(|(pos, &tid)| {
                        let rank = priority_order.iter().position(|r| r.vehicle_id == tid)?;
                        if rank > new_rank { Some(pos) } else { None }
                    })
                    .unwrap_or(self.traffic_order.len())
            });

        let insert_idx = if self.traffic_order.is_empty() { insert_idx } else { insert_idx.max(1) };

        self.traffic_order.insert(insert_idx, new_request.vehicle_id);
    }
}

impl IntersectionRequest {
    pub fn collisions_with(&self, others: &[IntersectionRequest], roads_count: usize) -> Vec<IntersectionRequest> {
        others.iter()
            .filter(|other| {
                other.vehicle_id != self.vehicle_id
                    && Intersection::paths_conflict(
                        self.entry_angle, self.exit_angle, self.arrival_time,
                        other.entry_angle, other.exit_angle, other.arrival_time,
                        roads_count,
                    )
            })
            .cloned()
            .collect()
    }
}

impl IntersectionController {
    // Détermine l'ordre de priorité parmi les requêtes conflictuelles.
    //
    // Hypothèses d'appel :
    //   - Les véhicules de type Stop ont déjà attendu.
    //   - Les véhicules TrafficLight sont au vert.
    //
    // Algorithme :
    //   1. Séparer en groupes priority vs yield.
    //   2. Appliquer la règle de priorité à droite dans chaque groupe.
    //   3. En cas d'égalité, départager par temps d'arrivée.
    fn determine_priority(requests: &[IntersectionRequest]) -> Vec<IntersectionRequest> {
        let mut priority_requests: Vec<&IntersectionRequest> = Vec::new();
        let mut yield_requests: Vec<&IntersectionRequest> = Vec::new();

        for req in requests {
            match req.rule {
                IntersectionRules::Priority => priority_requests.push(req),
                _ => yield_requests.push(req),
            }
        }

        let sort_by_right_priority = |a: &&IntersectionRequest, b: &&IntersectionRequest| {
            let delta = (b.entry_angle - a.entry_angle + 360.0) % 360.0;
            if delta > 180.0 {
                Greater
            } else if delta < 180.0 && delta > 0.0 {
                Less
            } else {
                a.arrival_time.partial_cmp(&b.arrival_time).unwrap_or(Equal)
            }
        };

        priority_requests.sort_by(sort_by_right_priority);
        yield_requests.sort_by(sort_by_right_priority);

        priority_requests.into_iter()
            .chain(yield_requests)
            .map(|r| r.clone())
            .collect()
    }
}
