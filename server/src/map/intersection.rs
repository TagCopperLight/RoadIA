use serde::{Deserialize, Serialize};
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum RoadRule {
    #[default]
    Priority,
    Yield,
    Stop,
}


#[derive(Clone)]
pub struct MovementRequest {
    pub vehicle_index: usize,
    pub vehicle_id: u64,
    pub to: NodeIndex,
    pub entry_angle: f64,
    pub exit_angle: f64,
    pub arrival_time: f32,
    pub rule: RoadRule,
}

impl MovementRequest {
    pub fn conflicts_with(&self, other: &MovementRequest, n_branches: usize) -> bool {//détecte si 2 véh veulent prendre la même sortie
        self.to == other.to
            || Intersection::paths_conflict(
                self.entry_angle,
                self.exit_angle,
                other.entry_angle,
                other.exit_angle,
                n_branches,
            )
    }
}

pub struct JunctionController; //Création de l'agent contrôleur de carrefour

impl JunctionController {
    pub fn new() -> Self {
        JunctionController
    }
    

    /// Retourne les indices des mouvements autorisés à s'engager
    pub fn authorized_indices(requests: &[MovementRequest], all_entry_angles: &[f64]) -> Vec<usize> {
        let mut allowed = Vec::new();
        
        let n = all_entry_angles.len();

        for (i, req) in requests.iter().enumerate() {
            let mut blocked = false;

            for (j, other) in requests.iter().enumerate() {
                if i == j { continue; }

                if !req.conflicts_with(other, n) {
                    continue;
                }

                // critere 1 : stop/céder le passage
                let my_rank = match req.rule {
                    RoadRule::Priority => 3,
                    RoadRule::Yield => 2,//céder le passage
                    RoadRule::Stop => 1,
                };
                let other_rank = match other.rule {
                    RoadRule::Priority => 3,
                    RoadRule::Yield => 2,
                    RoadRule::Stop => 1,
                };

                if other_rank > my_rank {
                    blocked = true;
                    break;
                }
                if my_rank > other_rank {
                    continue;
                }

                //critère 2 : quadrant
                let my_cost = Intersection::quadrant_cost(req.entry_angle, req.exit_angle, n);
                let other_cost = Intersection::quadrant_cost(other.entry_angle, other.exit_angle, n);

                if other_cost < my_cost {
                    blocked = true;
                    break;
                }
                
                if my_cost < other_cost {
                    continue;
                }

                //critère 3 : FIFO
                if other.arrival_time + 1e-3 < req.arrival_time {
                    blocked = true;
                    break;
                }

                if (req.arrival_time - other.arrival_time).abs() <= 1e-3
                    && other.vehicle_id < req.vehicle_id//critère 4 : ID véhicule (dernier recours, random possible sinon)
                {
                    blocked = true;
                    break;
                }
            }

            if !blocked {
                allowed.push(i);
            }
        }

        allowed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intersection {
    pub id: u32,
    pub kind: IntersectionKind,
    pub name: String,
    pub x: f32,
    pub y: f32,
    #[serde(default)]
    pub rules: HashMap<u32, RoadRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IntersectionKind {
    Habitation,
    Intersection,
    Workplace,
    Roundabout,
}

impl Intersection {
    pub fn compute_road_angle(&self, to: &Intersection) -> f64 {
        let dx = (to.x - self.x) as f64;
        let dy = (to.y - self.y) as f64;
        let angle_rad = dy.atan2(dx);
        let angle_deg = angle_rad.to_degrees();
        let mut heading = (450.0 - angle_deg) % 360.0;
        if heading < 0.0 { heading += 360.0; }
        heading
    }//conversion coordonnées -> angle de route

    pub fn paths_conflict(v1_entry: f64, v1_exit: f64, v2_entry: f64, v2_exit: f64, _n_branches: usize) -> bool {
        let (start1, end1) = Self::get_angular_interval(v1_entry, v1_exit);
        let (start2, end2) = Self::get_angular_interval(v2_entry, v2_exit);

        Self::intervals_conflict(start1, end1, start2, end2)
    }//vérification de conflits physiques

    pub fn get_angular_interval(entry: f64, exit: f64) -> (f64, f64) {
        (entry, exit)
    }

    fn intervals_conflict(entry1: f64, exit1: f64, entry2: f64, exit2: f64) -> bool {
        let range1 = Self::to_ranges(entry1, exit1);
        let range2 = Self::to_ranges(entry2, exit2);
        
        for (min1, max1) in &range1 {
            for (min2, max2) in &range2 {
                let overlap_start = min1.max(*min2);
                let overlap_end = max1.min(*max2);
                
                if overlap_end - overlap_start > 1.0 { 
                    return true;
                }
            }
        }
        false
    }//création des quadrants
   
    fn to_ranges(entry: f64, exit: f64) -> Vec<(f64, f64)> {
        let entry = (entry + 360.0) % 360.0;
        let exit = (exit + 360.0) % 360.0;
        
        if entry >= exit {
            vec![(exit, entry)]
        } else {
            vec![(0.0, entry), (exit, 360.0)]
        }
    }//faclilitation de la gestion des intervalles angulaires (problèmes de passage par 0°) 

    pub fn quadrant_cost(entry: f64, exit: f64, n_branches: usize) -> i32 {
        let diff = (entry - exit + 360.0) % 360.0;
        let sector_size = 360.0 / (n_branches as f64);
        
        // Arrondir au secteur le plus proche
        let cost = (diff / sector_size).round() as i32;
        if cost == 0 { n_branches as i32 } else { cost } //cout maximal = demi-tour
    }//Calcul du coût en quadrants d'un mouvement (premier critère de priorité)
}
