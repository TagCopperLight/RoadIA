# Architecture Frontend - Roadia

## 📋 Vue d'ensemble

Le frontend Roadia est une application **Next.js 16** avec **React 19** et **Pixi.js** pour le rendu graphique 2D. L'application utilise une architecture basée sur les **WebSockets** pour la communication bidirectionnelle temps réel avec le serveur Rust.

```
┌─────────────────────────────────────────────────────────────┐
│                    User Interface Browser                     │
│  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐   │
│  │   Toolbar    │  │   PropsPanel │  │   Legend        │   │
│  │  (Edit Mode) │  │  (Settings)  │  │  (Info & Keys)  │   │
│  └──────────────┘  └──────────────┘  └─────────────────┘   │
│         │                │                      │             │
│  ┌──────────────────────────────────────────────────────┐   │
│  │           MapComponent (Pixi.js Canvas)             │   │
│  │  - Render nodes & roads                             │   │
│  │  - Handle clicks & interactions                     │   │
│  │  - Display vehicles in real-time                    │   │
│  └──────────────────────────────────────────────────────┘   │
│                         │                                     │
└─────────────────────────┼─────────────────────────────────────┘
                          │
                    WebSocket Bridge
                          │
        ┌─────────────────┴─────────────────┐
        │                                   │
   [JSON Packets]                    [JSON Packets]
        │                                   │
        ▼                                   ▼
   OUTGOING                            INCOMING
   (Requests)                          (Updates)
   • addNode                           • map (state initiale)
   • updateNode                        • mapEdit (modifications)
   • deleteNode                        • vehicleUpdate (position véhicules)
   • addRoad
   • updateRoad
   • deleteRoad
   • startSimulation
   • stopSimulation
   • resetSimulation
```

---

## 🔌 1. WebSocket Communication (`app/websocket/websocket.tsx`)

### **Responsabilité**
Gère la **connexion WebSocket** et le **système de listeners** pour les paquets du serveur.

### **Classe: `WebSocketClient`**

#### **Constructeur et Connexion**
```typescript
constructor(url: string) → établit la connexion WebSocket au serveur
connect() → crée le socket et enregistre les événements
```

**Événements gérés:**
- `onopen` - Connexion établie, vide la queue des messages
- `onmessage` - Reçoit un paquet JSON `{ id: "type", data: {...} }`, l'envoie aux listeners
- `onclose` - Fermeture, tente une reconnexion automatique
- `onerror` - Erreur, notifie les listeners d'erreur et ferme la socket

#### **Système de Listeners**
```typescript
on(packetID, callback)     → Enregistre un listener pour un type de paquet
off(packetID, callback)    → Désenregistre un listener
dispatch(packetID, data)   → Exécute TOUS les listeners d'un type
```

**Exemple:**
```typescript
// Côté MapComponent
useWebSocket("map", (data) => {
    setMapData(data);  // Reçoit la map depuis le serveur
});

// Quand le serveur envoie: { id: "map", data: {...nodes, edges} }
// Le dispatch("map", data) appelle TOUS les listeners enregistrés
```

#### **Queue de Messages**
Si la socket n'est pas **encore connectée**:
```
send(packetID, data) → Message ajouté dans messageQueue
connexion établie     → flushQueue() envoie tous les messages
```

---

## 🎯 2. Context Global (`context/MapEditorContext.tsx`)

### **Responsabilité**
Partage **l'état global** entre tous les composants (état de simulation, sélection, toasts).

### **État partagé**

```typescript
interface MapEditorContextType {
    // Outil actif
    activeTool: 'pan' | 'select' | 'addNode' | 'addRoad'
    setActiveTool(tool) → Change l'outil actif
    
    // Sélection
    selectedNodeId: number | null      → ID du nœud sélectionné
    setSelectedNodeId(id)              → Sélectionne un nœud
    
    selectedEdgeId: number | null      → ID de la route sélectionnée
    setSelectedEdgeId(id)              → Sélectionne une route
    
    // État simulation
    isSimulating: boolean              → Est-ce que la simulation tourne?
    setIsSimulating(value)             → Lance/arrête la simulation
    
    // Notifications
    addToast(message, type, duration?) → Affiche une notification
}
```

**Utilisation dans les composants:**
```typescript
const { activeTool, selectedNodeId, isSimulating, addToast } = useMapEditor();
```

---

## 📱 3. Main Page Component (`components/MapPageClient.tsx`)

### **Responsabilité**
Composant racine de la page de map. Gère:
- **État de simulation** local
- **Sélection** (node/edge) locale
- **Raccourcis clavier** globaux (Delete, Escape)
- **Fournit le context** aux enfants

### **État local**

```typescript
const [activeTool, setActiveTool]         = useState('pan')
const [selectedNodeId, setSelectedNodeId] = useState(null)
const [selectedEdgeId, setSelectedEdgeId] = useState(null)
const [isSimulating, setIsSimulating]     = useState(false)
```

### **Raccourcis clavier**

Quand l'utilisateur appuie sur une touche (si pas dans un input):

```
DELETE/BACKSPACE → Supprime le nœud/route sélectionné
Escape           → Déselectionne tout (selectedNodeId = null)
```

### **Provider**

Enveloppe tous les enfants dans `MapEditorProvider` pour partager le context:

```typescript
<MapEditorProvider
    activeTool={activeTool}
    setActiveTool={setActiveTool}
    selectedNodeId={selectedNodeId}
    setSelectedNodeId={setSelectedNodeId}
    // ... autres props
>
    <Toolbar />
    <MapComponent />
    <ToastContainer />
</MapEditorProvider>
```

---

## 🎨 4. Toolbar Component (`components/Toolbar.tsx`)

### **Responsabilité**
Barre d'outils en haut pour changer **l'outil actif** et **contrôler la simulation**.

### **Outils disponibles**

```typescript
EDIT_TOOLS = [
    { tool: 'pan',     alt: 'Pan' }         // M
    { tool: 'select',  alt: 'Select' }      // V
    { tool: 'addNode', alt: 'Add Node' }    // N
    { tool: 'addRoad', alt: 'Add Road' }    // R
]
```

### **Actions**

```typescript
activeTool === tool
    → Applique l'outil (opacité 100%)
    → Autre outil (opacité 50%)

handlePlayPause()
    → Si EN COURS: stopSimulation(), setIsSimulating(false)
    → Si ARRÊTÉE: startSimulation(), setActiveTool('pan'), setIsSimulating(true)

handleReset()
    → resetSimulation() au serveur
    → setIsSimulating(false)
```

### **Raccourcis clavier**

```
M → activeTool = 'pan'
V → activeTool = 'select'
N → activeTool = 'addNode'
R → activeTool = 'addRoad'
```

---

## 🗺️ 5. Map Component (`components/MapComponent.tsx`)

### **Responsabilité**
Composant principal de la map. Gère:
- Réception de l'état de la map du serveur
- Passage des données à Pixi.js (MapCanvas)
- Affichage du PropertiesPanel quand quelque chose est sélectionné
- Affichage de la Legend

### **Flux de données**

```
1. Montage du composant
   ↓
   sendConnectionToken("auth-token") → WebSocket au serveur
   ↓
2. Serveur envoie "map" paquet
   ↓
   useWebSocket("map", (data) => setMapData(data))
   ↓
   mapData = { nodes: [...], edges: [...] }
   ↓
3. <PixiApp mapData={mapData} /> → Affiche la map interactive
```

### **Écoutes WebSocket**

```typescript
// Au démarrage: reçoit l'état COMPLET de la map
useWebSocket("map", (data) => {
    setMapData(data);  // { nodes, edges }
});

// Quand quelque chose est modifié: reçoit la map mise à jour
useWebSocket("mapEdit", (data) => {
    if (data.success) {
        setMapData({ nodes: data.nodes, edges: data.edges });
        
        // Détecte les nouveaux nœuds Habitation/Workplace
        // et crée automatiquement des véhicules
        newNodes.forEach(node => {
            sendPacket('createVehicle', { origin_id: node.id });
        });
    }
});

// Mise à jour CONTINUE des positions des véhicules
useWebSocket("vehicleUpdate", (data) => {
    setVehicles(data.vehicles);  // Redessine every frame
});
```

### **Interaction avec la map**

```typescript
<PixiApp
    sendPacket={(action, data) => wsClient.send(action, data)}
    onUpdateEdge={(id, lanes, speed, type) => 
        sendPacket('updateRoad', {id, lane_count: lanes, speed_limit: speed})
    }
/>
```

---

## 🎮 6. Canvas - Pixi.js Rendering (`components/map/MapCanvas.tsx`)

### **Responsabilité**
**Rendu et interaction** avec la map Pixi.js:
- Affiche visuellement les nœuds et routes
- Gère les clics et le drag
- Utilise un **viewport** pour zoom/pan
- Détecte les collisions pour la sélection

### **Contenu affiché**

```typescript
for each node:
    <Intersection node={node} /> → Cercle coloré + icône
    
for each road:
    <Road edge={edge} />          → Ligne entre deux nœuds
    
for each vehicle:
    <Vehicle vehicle={vehicle} /> → Petit carré en mouvement
```

### **Interaction - Outils actifs**

**Tool: 'pan'** (zoom/panoramique)
```
Mouse Down + Move → Déplace le viewport (camera)
Mouse Wheel       → Zoom in/out
```

**Tool: 'select'** (sélectionner)
```
Click on node  → setSelectedNodeId(nodeId)
Click on road  → setSelectedEdgeId(edgeId)
Click vide     → setSelectedNodeId(null)
```

**Tool: 'addNode'** (créer nœud)
```
Click vide → sendPacket('addNode', {x, y, kind: 'Intersection', name: 'node_N'})
```

**Tool: 'addRoad'** (créer route)
```
Click node 1 → setAddRoadSource(node1.id)  (attendre destination)
Click node 2 → sendPacket('addRoad', {from_id: node1, to_id: node2})
         OU
Click node 1 → show toast "Select destination node"
```

### **Vérifications de validation**

Avant d'envoyer l'action au serveur:
```typescript
if (isSimulating) {
    addToast('Stop the simulation to edit the map', 'warning')
    return;  // N'envoie PAS l'action
}

validateNodeCreation(x, y, name, kind)  // Lance ValidationError si invalide
validateDifferentNodes(fromId, toId)    // Lance ValidationError si même nœud
```

### **État utilisé**

```typescript
const { activeTool, selectedNodeId, setSelectedNodeId, isSimulating, addToast } = useMapEditor();
const { app } = useApplication();  // Pixi app instance

// Drag de nœuds
const [draggingNodeId, setDraggingNodeId]  = useState(null)
const [dragPos, setDragPos]                = useState(null)

// Pré-sélection pour addRoad
const [addRoadSource, setAddRoadSource]    = useState(null)
```

---

## 📋 7. Properties Panel (`components/PropertiesPanel.tsx`)

### **Responsabilité**
Affiche le **formulaire de propriétés** pour le nœud/route sélectionné.
Permet l'édition avec un système **d'Apply + Cancel**.

### **État du formulaire**

**Pour les nœuds:**
```typescript
const [nodeName, setNodeName]           = useState('')
const [nodeKind, setNodeKind]           = useState('Intersection')
const [hasNodeChanges, setHasNodeChanges] = useState(false)
```

**Pour les routes:**
```typescript
const [laneCount, setLaneCount]         = useState(1)
const [speedLimit, setSpeedLimit]       = useState(40)
const [intersectionType, setIntersectionType] = useState('Priority')
const [hasEdgeChanges, setHasEdgeChanges] = useState(false)
```

### **Flux de modification**

```
1. Sélectionne un nœud
   ↓
   (useEffect) → setNodeName(selectedNode.name), setHasNodeChanges(false)
   
2. L'utilisateur change la valeur (ex: setNodeName("new_name"))
   ↓
   setHasNodeChanges(true)
   
3. Affiche les boutons "Apply" et "Cancel"
   
4a. Clique "Apply"
    ↓
    handleNodeCommit()
    ↓
    Valide le nom (pas vide, pas dupliqué)
    ↓
    onUpdateNode(id, nodeKind, nodeName)  → sendPacket('updateNode', {...})
    ↓
    setHasNodeChanges(false) → cache les boutons
    
4b. Clique "Cancel"
    ↓
    handleNodeCancel()
    ↓
    setNodeName(selectedNode.name)  → revient à la valeur d'avant
    ↓
    setHasNodeChanges(false) → cache les boutons
```

### **Vérifications avant Apply**

```typescript
if (isSimulating) {
    addToast('Stop the simulation to edit the map', 'warning')
    return;
}

// Pour les nœuds
validateNodeName(name)  → Vérifie pas vide, pas dupliqué
```

### **Désactivation pendant simulation**

```typescript
<input disabled={isSimulating} />   // L'input ne peut pas être modifié
className={isSimulating ? 'opacity-50 cursor-not-allowed' : ''}
```

---

## 🔍 8. Legend Component (`components/Legend.tsx`)

### **Responsabilité**
Affiche une **légende rétractable** avec:
- Explication des couleurs des nœuds
- Explication des types de routes
- Tous les raccourcis clavier

### **État**

```typescript
const [isExpanded, setIsExpanded] = useState(true)

// Click sur le header toggle
onClick={() => setIsExpanded(!isExpanded)}
```

### **Contenu**

**Nœuds:**
- 🔵 Blue = Intersection
- 🟠 Orange = Workplace
- 🟢 Green = Habitation
- 🟣 Purple = Roundabout
- 🔴 Red = Traffic Light

**Routes:**
- ─────── Standard (gray)
- ─────── Blocked (red)

**Raccourcis:**
```
M → Pan
V → Select
N → Add Node
R → Add Road
DEL → Delete selected
ESC → Deselect
```

---

## 📡 Flux global des données

### **Initialisation**

```
1. User charge la page
   ↓
2. MapPageClient monte
   ↓
3. MapComponent monte
   ↓
4. sendConnectionToken("auth-token")
   ↓
5. Serveur envoie "map" paquet avec { nodes, edges }
   ↓
6. useWebSocket("map") saisit et setMapData()
   ↓
7. PixiApp reçoit mapData et affiche la map
```

### **Créer un nœud**

```
1. activeTool = 'addNode'
2. User click sur la canvas
   ↓
3. MapCanvas détecte click vide
   ↓
4. Valide position et nom
   ↓
5. sendPacket('addNode', {x, y, kind: 'Intersection', name: 'node_5'})
   ↓
6. Serveur traite et envoie 'mapEdit' paquet
   ↓
7. useWebSocket("mapEdit") reçoit nouvelle map
   ↓
8. setMapData() → Pixi re-affiche avec le nouveau nœud
   ↓
9. Si c'est Habitation ou Workplace → auto-crée un véhicule
```

### **Modifier une route (lane count ou speed)**

```
1. User sélectionne une route
   ↓
2. PropertiesPanel affiche les champs
   ↓
3. User change lane_count = 3
   ↓
4. setLaneCount(3)
   ↓
5. setHasEdgeChanges(true) → affiche buttons
   ↓
6. Clique "Apply"
   ↓
7. handleEdgeCommit()
   ↓
8. sendPacket('updateRoad', {id, lane_count: 3, speed_limit: 30})
   ↓
9. Serveur envoie 'mapEdit'
   ↓
10. La route s'affiche avec les nouvelles propriétés
```

### **Simulation en cours**

```
1. User clique Play
   ↓
2. handlePlayPause()
   ↓
3. sendPacket('startSimulation', {})
   ↓
4. setIsSimulating(true)
   ↓
5. Tous les inputs/buttons sont disabled
   ↓
6. Si user tente d'éditer → toast "Stop simulation"
   ↓
7. Serveur envoie 'vehicleUpdate' toutes les frames
   ↓
8. Pixi re-affiche les véhicules à leurs nouvelles positions
   ↓
9. User clique Pause
   ↓
10. sendPacket('stopSimulation', {})
    ↓
11. setIsSimulating(false) → inputs re-activés
```

---

## 🔗 Résumé des Paquets WebSocket

### **Envoyés par le Frontend**

| Paquet | Données | Quand |
|--------|---------|-------|
| `connect` | `{ token: string }` | Démarrage |
| `addNode` | `{ x, y, kind, name }` | Tool addNode + click |
| `updateNode` | `{ id, kind, name }` | Properties panel apply |
| `deleteNode` | `{ id }` | Delete key ou button |
| `moveNode` | `{ id, x, y }` | Drag nœud relâché |
| `addRoad` | `{ from_id, to_id, lane_count, speed_limit }` | Tool addRoad + 2e click |
| `updateRoad` | `{ id, lane_count, speed_limit, intersection_type }` | Properties panel apply |
| `deleteRoad` | `{ id }` | Delete key ou button |
| `startSimulation` | `{}` | Play button |
| `stopSimulation` | `{}` | Pause button |
| `resetSimulation` | `{}` | Reset button |
| `createVehicle` | `{ origin_id }` | Auto-création après node add |

### **Reçus du Serveur**

| Paquet | Contenu | Quand |
|--------|---------|-------|
| `map` | `{ nodes: [], edges: [] }` | Démarrage / reconnexion |
| `mapEdit` | `{ success, nodes, edges, error? }` | N'importe quelle édition de map |
| `vehicleUpdate` | `{ vehicles: [] }` | Chaque frame pendant simulation |

---

## 💾 Résumé complet

**Frontend = Interface HTML + Pixi.js Rendering + WebSocket Bridge**

1. **WebSocket** = Communication avec le serveur (bidirectionnelle, event-based)
2. **Context** = Partage d'état global entre composants
3. **MapPageClient** = Root component, gère raccourcis clavier globaux
4. **Toolbar** = Sélection d'outils + contrôle simulation
5. **MapComponent** = Reçoit data serveur, passe à PixiApp
6. **PixiApp + MapCanvas** = Rendu et interaction avec la map
7. **PropertiesPanel** = Édition des propriétés en Apply+Cancel
8. **Legend** = Infos utilisateur rétractable

**Tout est réactif via WebSocket** - quand le serveur envoie une mise à jour, React re-rend automatiquement.
