use crate::map::model::Map;
use crate::map::intersection::{Intersection, IntersectionKind, RoadRule};
use crate::map::road::Road;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

pub struct Roundabout {
    pub center_x: f32,
    pub center_y: f32,
    pub radius: f32,
}

impl Roundabout {
    pub fn new(x: f32, y: f32, radius: f32) -> Self {
        Self {
            center_x: x,
            center_y: y,
            radius,
        }
    }

    /// Génère le rond-point en créant une intersection pour chaque route connectée.
    /// `connections` est une liste de tuples (Intersection externe, route entrante, route sortante)
    /// Mais pour simplifier, disons que l'on connecte des voisins existants.
    ///
    /// Pour l'instant, faisons une méthode qui prend une liste de points (intersections voisines)
    /// et construit l'anneau.
    pub fn build(&self, map: &mut Map, connected_intersections: Vec<NodeIndex>) -> Vec<NodeIndex> {
        if connected_intersections.is_empty() {
            return vec![];
        }

        let mut ring_nodes = Vec::new();
        let mut node_angles = Vec::new();

        // 1. Créer les intersections sur l'anneau (les "T")
        for (i, &neighbor_idx) in connected_intersections.iter().enumerate() {
            let neighbor = &map.graph[neighbor_idx];
            
            // Calculer l'angle vers le voisin pour placer le noeud du rond-point face à lui
            let dx = neighbor.x - self.center_x;
            let dy = neighbor.y - self.center_y;
            let angle = dy.atan2(dx); // radians

            let px = self.center_x + self.radius * angle.cos();
            let py = self.center_y + self.radius * angle.sin();

            let ring_node_id = map.graph.node_count() as u32 + 1000; // ID temporaire/unique

            let node = map.add_intersection(Intersection {
                id: ring_node_id, // TODO: gestion d'ID plus propre
                kind: IntersectionKind::Roundabout,
                name: format!("RB-Node-{}", i),
                x: px,
                y: py,
                rules: HashMap::new(),
            });

            ring_nodes.push(node);
            node_angles.push((angle, node, neighbor_idx));
        }

        // Trier les noeuds par angle pour former le cercle correctement (sens trigo ou horaire)
        // On suppose sens anti-horaire (trigo) pour rond-point standard (conduite à droite = sens anti-horaire)
        node_angles.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        // 2. Connecter l'anneau (Barre du T) avec sens unique
        let n = node_angles.len();
        for i in 0..n {
            let (_angle_current, current_node, neighbor_idx) = node_angles[i];
            let (_angle_next, next_node, _next_neighbor) = node_angles[(i + 1) % n];

            // Lien Anneau -> Anneau (Prioritaire)
            // On calcule la distance approximative ou on met une valeur fixe
            // Pour un cercle, la distance corde est 2*R*sin(dtheta/2)
            // Mais road.rs calcule souvent ses propres longueurs ou on les donne.
            let road_ring = Road::new(
                    999, // ID à gérer
                    1, 
                    8, // ~30 km/h dans le rond point
                    20.0, // Longueur approx
                    false, 
                    false
            );
            
            // Créer l'arête anneau i -> anneau i+1
            map.add_road(current_node, next_node, road_ring.clone());

            // 3. Connecter le "Pied du T" (Entrée/Sortie avec le voisin)
            
            // Sortie du rond point (Anneau -> Voisin) : Priorité standard (ou sortie libre?)
            // Généralement sortir du rond point est prioritaire par rapport à ceux qui veulent y entrer non?
            // Non, c'est juste une sortie.
            
            let road_out = Road::new(998, 1, 13, 50.0, false, false);
            let edge_out = map.add_road(current_node, neighbor_idx, road_out);
            
            // Entrée dans le rond point (Voisin -> Anneau) : CÉDEZ LE PASSAGE
            let road_in = Road::new(997, 1, 13, 50.0, false, false);
            let edge_in = map.add_road(neighbor_idx, current_node, road_in);

            // APPLIQUER LA RÈGLE SUR L'INTERSECTION DU ROND POINT
            // L'intersection est `current_node`.
            // La route entrante est `road_in` (via `edge_in`). -> Yield
            // La route venant de l'anneau (précédent -> current) -> Priority

            // On a besoin de l'ID de la route entrante pour la map de règles.
            // map.add_road consomme le road object, mais on peut récupérer la ref ou prédéfinir les IDs.
            // Ici j'ai mis des ID bidon, il faudrait un générateur d'ID global.
            
            // Pour faire propre, récupérons l'ID de la route entrante graphiquement
            let incoming_road_id = map.graph[edge_in].id;
            
            // Récupérer le noeud pour modifier ses règles
            if let Some(inter) = map.graph.node_weight_mut(current_node) {
                 inter.rules.insert(incoming_road_id, RoadRule::Yield);
                 // La route venant de l'anneau (le précédent) aura Priority par défaut (Enum default)
            }
        }
        
        // Retourne les noeuds créés si besoin
        ring_nodes
    }
}
