use serde::{Deserialize, Serialize};
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
pub enum RoadRule {
    #[default]
    Priority,//par défaut
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
    } // End of quadrant_cost
} // End of impl Intersection

use crate::map::model::Map;
use crate::map::road::Road;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Roundabout {
    pub id: u32,
    pub name: String,
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
}

impl Roundabout {
    pub fn new(id: u32, name: String, x: f32, y: f32, radius: f32) -> Self {//radius = rayon en mètre pour la taille physique
        Self {
            id,
            name,
            center_x: x,
            center_y: y,
            radius,
        }
    }
    
    pub fn build(&self, map: &mut Map, connected_intersections: Vec<NodeIndex>) -> Vec<NodeIndex> {
        if connected_intersections.is_empty() {//cas limite : pas de branches
            return vec![];
        }

        let mut ring_nodes = Vec::new();//liste des noeuds de l'anneau
        let mut node_angles = Vec::new();

        for (i, &neighbor_idx) in connected_intersections.iter().enumerate() {//création des noeuds de l'anneau sous formes d'intersections
            let neighbor = &map.graph[neighbor_idx];
            let dx = neighbor.x - self.center_x;
            let dy = neighbor.y - self.center_y;
            let angle = dy.atan2(dx);//angle entre le centre du rond-point et le voisin

            let px = self.center_x + self.radius * angle.cos();//coordonnées du noeud sur l'anneau en x
            let py = self.center_y + self.radius * angle.sin();//coordonnées du noeud sur l'anneau en y

            let ring_node_id = self.id * 1000 + (i as u32); //génération id pour la nouvelle intersection

            let node = map.add_intersection(Intersection {
                id: ring_node_id,
                kind: IntersectionKind::Roundabout, //marque l'appartenance au rond-point
                name: format!("{}-Node-{}", self.name, i), //nommage descriptif avec id
                x: px,
                y: py,
                rules: HashMap::new(),
            });

            ring_nodes.push(node);
            node_angles.push((angle, node, neighbor_idx));
        }

        node_angles.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());//tri angulaire pour garantir le sens anti-horaire

        let n = node_angles.len();
        for i in 0..n {
            let (_angle_current, current_node, neighbor_idx) = node_angles[i];
            let (_angle_next, next_node, _next_neighbor) = node_angles[(i + 1) % n];

            //1) création du noeud interne de l'anneau
            
            //calcul delta_theta pour longeur de la route
            let mut d_theta = _angle_next - _angle_current;
            if d_theta <= 0.0 {
                d_theta += 2.0 * std::f32::consts::PI as f32; // On boucle le cercle si on passe 0 radian
            }

            let arc_length = (self.radius * d_theta).max(5.0);//calcul longueur de l'arc avec sécurité à 5m (évite blocage véh si pas assez de place)

            let road_ring = Road::new(
                    self.id * 10000 + (i as u32), //génération id (à revoir si grosse ville)
                    1, //1 seule voie (à adapter aux rond-points, voir plus tard avec import de map)
                    8, //30km/h (à discuter si vitesse trop forte car dépends de la taille du rond-point, utilisation de formule d'adhérence possible)
                    arc_length, //longueur calculée précisément
                    false,
                    false  //pas de dépassement (à modifier)
            );
            map.add_road(current_node, next_node, road_ring);

            //2) création des routes entrantes
            let road_in = Road::new(self.id * 20000 + (i as u32), 1, 13, 50.0, false, false);
            let edge_in = map.add_road(neighbor_idx, current_node, road_in);
            
            //3) application de la règle de priorité
            let incoming_road_id = map.graph[edge_in].id;
            if let Some(inter) = map.graph.node_weight_mut(current_node) {
                 inter.rules.insert(incoming_road_id, RoadRule::Yield);
            }

            //4) création des routes sortantes
            let road_out = Road::new(self.id * 30000 + (i as u32), 1, 13, 50.0, false, false);
            map.add_road(current_node, neighbor_idx, road_out);
        }
        
        ring_nodes//retourne la liste des noeuds créés
    }
}
