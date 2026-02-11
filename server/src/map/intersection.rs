use petgraph::graph::NodeIndex;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum RoadRule {
    #[default]
    Priority,//par défaut
    Yield,
    Stop,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrafficLightColor {
    Red,
    Orange,
    Green,
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
    pub light_color: Option<TrafficLightColor>,
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
    /// Retourne les indices des mouvements autorisés à s'engager
    pub fn authorized_indices(requests: &[MovementRequest], all_entry_angles: &[f64]) -> Vec<usize> {
        let mut allowed = Vec::new();
        let n = all_entry_angles.len();

        for (i, req) in requests.iter().enumerate() {
            // 1) Feux tricolores
            if let Some(color) = req.light_color {
                if color == TrafficLightColor::Red || color == TrafficLightColor::Orange {
                     continue; 
                }
            }

            let mut blocked_reason: Option<String> = None;

            for (j, other) in requests.iter().enumerate() {
                if i == j { continue; }
                if blocked_reason.is_some() { break; } 

                // 1) Feux tricolores (actuellement un vert)
                 if let Some(color) = other.light_color {
                    if color == TrafficLightColor::Red || color == TrafficLightColor::Orange { continue; }
                }

                // 2) régle de priorité (priority > Yield > Stop)
                let my_rank = match req.rule {
                    RoadRule::Priority => 3,
                    RoadRule::Yield => 2,    // Cédez le passage
                    RoadRule::Stop => 1,
                };
                let other_rank = match other.rule {
                    RoadRule::Priority => 3,
                    RoadRule::Yield => 2,
                    RoadRule::Stop => 1,
                };

                if other_rank > my_rank {//gestion des conflits (autres prioritaires)
                    if req.conflicts_with(other, n) {
                        blocked_reason = Some(format!("Rank (V{} {:?} > V{} {:?}) & Physical Conflict", 
                            other.vehicle_id, other.rule, req.vehicle_id, req.rule));
                        break;
                    }
                    continue; 
                } 
                
                if my_rank > other_rank {//gestion des conflits (je suis prioritaire)
                    continue;
                }

                // 3) conflit physique (si rangs égaux)
                if !req.conflicts_with(other, n) {
                    continue;
                }

                // 4) priorité à droite
                let delta = (other.entry_angle - req.entry_angle + 360.0) % 360.0;// calcul de la position relative de l'autre véhicule
                
                if delta > 180.0 + 1e-3 {//angle>180 => viens de droite => je suis prioritaire 
                    blocked_reason = Some(format!("Right Hand Priority (Delta {:.1}°)", delta));
                    break;
                }
                if delta < 180.0 - 1e-3 {//angle<180 => viens de gauche => l'autre est prioritaire
                    continue;
                }

                // 5) FIFO/ID (random)
                if other.arrival_time + 1e-3 < req.arrival_time {
                     blocked_reason = Some(format!("FIFO (V{} arrived first)", other.vehicle_id));
                     break;
                }

                if (req.arrival_time - other.arrival_time).abs() <= 1e-3
                    && other.vehicle_id < req.vehicle_id 
                {
                     blocked_reason = Some(format!("ID Tie-Breaker (V{} < V{})", other.vehicle_id, req.vehicle_id));
                     break;
                }
            }

            if blocked_reason.is_none() {
                 allowed.push(i);
            }
        }
        
        // 6) gestion interblocage
        if allowed.is_empty() && !requests.is_empty() {
             let candidates: Vec<(usize, &MovementRequest)> = requests.iter().enumerate()
                .filter(|(_, r)| match r.light_color {
                    Some(TrafficLightColor::Red) | Some(TrafficLightColor::Orange) => false,
                    _ => true
                })
                .collect();
             
             if !candidates.is_empty() {
                 if let Some((best_idx, _)) = candidates.iter().min_by(|(_, a), (_, b)| {
                        a.arrival_time.partial_cmp(&b.arrival_time).unwrap_or(std::cmp::Ordering::Equal)
                            .then_with(|| a.vehicle_id.cmp(&b.vehicle_id))
                    }) 
                 {
                     allowed.push(*best_idx); // Ce véhicule force le passage pour débloquer le carrefour
                 }
             }
        }

        allowed
    }
}

#[derive(Debug, Clone)]
pub struct Intersection {
    pub id: u32,
    pub kind: IntersectionKind,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub rules: HashMap<u32, RoadRule>,
    
    //feu tricolore
    pub traffic_lights: HashMap<u32, TrafficLightColor>,
    pub timer: f32,
    pub current_green_idx: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IntersectionKind {
    Habitation,
    Intersection,
    Workplace,
    Roundabout,
    TrafficLight,
}

impl Intersection {
    pub fn new(id: u32, kind: IntersectionKind, name: String, x: f32, y: f32) -> Self {
        Self {
            id,
            kind,
            name,
            x,
            y,
            rules: HashMap::new(),
            traffic_lights: HashMap::new(),
            timer: 0.0,
            current_green_idx: 0,
        }
    }

    pub fn update_traffic_lights(&mut self, dt: f32, incoming_roads: &[u32]) {//gestion feu tricolore
        if self.kind != IntersectionKind::TrafficLight {
            return;
        }
        if incoming_roads.is_empty() { return; }
        let active_roads = incoming_roads;
        self.timer += dt;
        let green_duration = 10.0;
        let orange_duration = 3.0;
        let red_clearance = 2.0;
        let cycle_step_duration = green_duration + orange_duration + red_clearance;
        if self.timer > cycle_step_duration {
            self.timer = 0.0;
            self.current_green_idx = (self.current_green_idx + 1) % active_roads.len();
        }
        let current_road_id = active_roads[self.current_green_idx];
        for road_id in active_roads {
             if *road_id == current_road_id {
                 if self.timer < green_duration {
                     self.traffic_lights.insert(*road_id, TrafficLightColor::Green);
                 } else if self.timer < green_duration + orange_duration {
                     self.traffic_lights.insert(*road_id, TrafficLightColor::Orange);
                 } else {
                     self.traffic_lights.insert(*road_id, TrafficLightColor::Red);
                 }
             } else {
                 self.traffic_lights.insert(*road_id, TrafficLightColor::Red);
             }
        }
    }

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
}

use crate::map::model::Map;
use crate::map::road::Road;

#[derive(Debug, Clone, PartialEq)]
pub enum RoundaboutKind {
    Standard, //rdp
    Gyratory, //carrefour à sens giratoire
}

#[derive(Debug, Clone)]
pub struct Roundabout {
    pub id: u32,
    pub kind: RoundaboutKind,
    pub name: String,
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
}

impl Roundabout {
    pub fn new(id: u32, name: String, x: f32, y: f32, radius: f32, kind: RoundaboutKind) -> Self {
        Self {
            id,
            kind,
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

            let node = map.add_intersection(Intersection::new(
                ring_node_id,
                IntersectionKind::Roundabout,
                format!("{}-Node-{}", self.name, i),
                px,
                py,
            ));

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
            
            // Id de la route de l'anneau qui arrive sur ce noeud (pour le mode Gyratory)
            let prev_idx = (i + n - 1) % n;
            let incoming_ring_road_id = self.id * 10000 + (prev_idx as u32);//id à revoir si grosse ville

            if let Some(inter) = map.graph.node_weight_mut(current_node) {
                 match self.kind {
                     RoundaboutKind::Standard => {
                         inter.rules.insert(incoming_road_id, RoadRule::Yield);//cédez le passage pour rdp
                     },
                     RoundaboutKind::Gyratory => {//donne règle giratoire
                         inter.rules.insert(incoming_ring_road_id, RoadRule::Yield);
                         inter.rules.insert(incoming_road_id, RoadRule::Priority);
                     }
                 }
            }

            //4) création des routes sortantes
            let road_out = Road::new(self.id * 30000 + (i as u32), 1, 13, 50.0, false, false);
            map.add_road(current_node, neighbor_idx, road_out);
        }
        
        ring_nodes//retourne la liste des noeuds créés
    }
}
