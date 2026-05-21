# Map editor

Fichier principal: `src/map/editor.rs`.

Ce chapitre documente les fonctions de modification de la carte utilisées par l'interface HTTP et le WebSocket.

## Fonctions

### `add_node`

Ajoute un noeud au graphe avec les coordonnées fournies et un `IntersectionKind`. La fonction renvoie l'identifiant public du noeud créé.

### `delete_node`

Supprime un noeud par son identifiant, met à jour la table des indices et reconstruit les intersections. Si le noeud est absent, la fonction renvoie une erreur textuelle.

### `update_node`

Change uniquement le type fonctionnel d'un noeud. La topologie routière n'est pas reconstruite automatiquement par cette fonction; elle doit être recalculée si le changement de rôle a un impact sur les trajets.

### `add_road`

Crée une route orientée entre deux noeuds existants. La longueur utile est calculée à partir de la distance géométrique entre les centres, en retirant les rayons des intersections pour obtenir une longueur exploitable par les véhicules. La topologie d'intersection est ensuite reconstruite.

### `delete_road`

Supprime une route existante par son identifiant, puis reconstruit les intersections pour supprimer les liens devenus invalides.

### `update_road`

Met à jour la vitesse limite d'une route et, si demandé, son nombre de voies. Les voies sont recréées proprement afin de repartir d'un état cohérent, puis la topologie d'intersection est recalculée.

### `add_roundabout`

Construit un rond-point en créant un anneau de noeuds et de routes autour d'un centre géométrique. Le résultat est encapsulé dans un `RoundaboutHandle` qui contient les identifiants des noeuds et des routes de l'anneau.

Contraintes importantes:

- le nombre d'armes doit être au moins 3;
- le rayon doit être strictement positif;
- le nombre de voies de l'anneau doit être au moins 1;
- le rayon doit être assez grand pour laisser la place physique aux bras du rond-point.

### `add_traffic_light_controller`

Crée un contrôleur de feux sur une intersection donnée.

Les phases sont fournies comme une liste de triplets: identifiants de liens verts, durée du vert, durée du jaune. La fonction marque les liens concernés comme `TrafficLight` et ajoute le contrôleur dans la carte.

## Ce qu'il faut retenir

- Les fonctions de ce module sont celles qui modifient réellement la topologie.
- Elles doivent être suivies d'une reconstruction des intersections dès qu'elles changent la connectivité ou les priorités.
- Les handles retournés par les helpers complexes servent à finaliser correctement la topologie après coup.