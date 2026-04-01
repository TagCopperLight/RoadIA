# 📋 Code Review Frontend - Roadia Map Editor

**Date:** 26 Mars 2026  
**Scope:** Next.js Client Application  
**Status:** ✅ Functional, Ready for Improvements

---

## 📊 Table des Matières

1. [Vue d'Ensemble](#vue-densemble)
2. [Architecture](#architecture)
3. [Points Forts](#points-forts)
4. [Points Faibles & Améliorations](#points-faibles--améliorations)
5. [Recommandations Prioritaires](#recommandations-prioritaires)

---

## 👀 Vue d'Ensemble

### Stack Technologique
- **Framework:** Next.js 16.1.4 (Turbopack)
- **UI Framework:** React 19.2.3
- **Graphics:** Pixi.js v8.15.0 + @pixi/react v8.0.5
- **Styling:** Tailwind CSS v4
- **Communication:** WebSocket (custom client)
- **Typing:** TypeScript 5.x

### Structure Générale
```
Home Page (menu) 
    ↓
Map Page [uuid]
    ↓
MapPageClient (orchestrator, état global)
    ↓
MapComponent (données + websocket)
    ↓
PixiApp (Pixi.js application)
    ↓
MapCanvas (logique d'édition + hit testing)
    ↓
Intersection, Road, Vehicle (éléments)
```

---

## 🏗️ Architecture

### 1. **Page Layer** (`app/`)
- **page.tsx** - Home menu simple avec navigation
- **map/[uuid]/page.tsx** - Page de la carte avec Header
- Utilise composants serveur pour imprimer les données statiques

**Verdict:** ✅ Bon - Séparation claire page/logique

### 2. **Client Layer** (`components/MapPageClient.tsx`)

**Responsabilités:**
- État global: `editMode`, `activeTool`, `selectedNodeId/EdgeId`
- Toast notifications (hook `useToast`)
- Keyboard shortcuts (Delete, Backspace, Escape)
- Liaison WebSocket

**État:**
```typescript
const [editMode, setEditMode] = useState(false);
const [activeTool, setActiveTool] = useState<EditTool>('select');
const [selectedNodeId, setSelectedNodeId] = useState<number | null>(null);
const [selectedEdgeId, setSelectedEdgeId] = useState<number | null>(null);
```

**Refs Utiles:**
```typescript
// Évite "stale closure" dans les event listeners
const selectedNodeIdRef = useRef(selectedNodeId);
selectedNodeIdRef.current = selectedNodeId;
```

**Verdict:** ✅ Bon - Centralisé et bien structuré

### 3. **Map Component** (`components/MapComponent.tsx`)

**Responsabilités:**
- Gestion de la connexion WebSocket
- Récception des données de carte et véhicules
- Props drilling vers PixiApp
- PropertiesPanel (panneau latéral d'édition)

**Données gérées:**
```typescript
const [mapData, setMapData] = useState<MapData | null>(null);
const [vehicles, setVehicles] = useState<VehicleData[]>([]);
```

**WebSocket Integration:**
```typescript
useWebSocket("map", (data) => setMapData(data as MapData));
useWebSocket("mapEdit", useCallback((data: any) => { ... }, []));
```

**Verdict:** ⚠️ Passable - Fonctionne mais dense avec logique mélangée

### 4. **Pixi Application** (`components/map/PixiApp.tsx`)

**Rôle:** Initialisation de Pixi.js Application et rendu

```typescript
<Application onInit={handleInit} background={0xC1D9B7} resizeTo={resizeTo}>
  {isInitialized && mapData && (
    <MapCanvas {...props} />
  )}
</Application>
```

**Verdict:** ✅ Bon - Rôle unique et clair

### 5. **Map Canvas** (`components/map/MapCanvas.tsx`) - ⭐ **Cœur du Système**

**Responsabilités Critiques:**
- Hit testing (nodes et edges)
- Event handling (pointermove, pointerup, click)
- Gestion du drag de nodes
- Mode addNode et addRoad
- Rubber-band line pour visual feedback
- Rendu des nodes, edges et véhicules

**Architecture d'Event Handling:**
```typescript
// Au niveau du stage (viewport) pour éviter issues d'event propagation
app.stage.on('pointermove', onMove);
app.stage.on('pointerup', onUp);
app.stage.on('click', onClick);

// Manual hit testing
interface hitTestNode(worldX, worldY, node) {
  const dx = worldX - node.x;
  const dy = worldY - node.y;
  return dx*dx + dy*dy <= 16*16; // ⚠️ Magic number!
}
```

**Optimisation de Render:**
```typescript
// Instead of setState on every move:
const pointerPosRef = useRef<{ x: number; y: number } | null>(null);
const [, setPointerPosState] = useState(0); // Dummy state for forced renders

// In onMove:
pointerPosRef.current = worldPos; // No re-render
if (activeTool === 'addNode') {
  setPointerPosState(prev => prev + 1); // Force render si needed
}
```

**Verdict:** ✅ Fort - Bien optimisé, logique claire

### 6. **UI Components**

#### ✅ Toolbar.tsx
- Tool selection: select, addNode, addRoad
- Simulation controls: play, pause, reset
- Keyboard shortcuts: E=edit, V=select, N=addNode, R=addRoad
- Bien implémenté avec toggles visuels

#### ✅ PropertiesPanel.tsx
- Édition node: name, kind (Intersection/Habitation/Workplace)
- Édition edge: lane_count, speed_limit, is_blocked, can_overtake
- Auto-commit on blur ou Enter
- À droite de l'écran (fixed position)

#### ⚠️ Intersection & Road Components
- Simples composants de rendu
- Props bien typées
- Event handlers directs (onSelect, onDragStart)

### 7. **Custom Hooks**

#### 🟢 `useToast()`
```typescript
const { toasts, addToast, removeToast } = useToast();
```
- Auto-dismiss après X ms
- ID uniques pour chaque toast
- Intégré partout: node add/move, road add, etc.

**Verdict:** ✅ Excellent - Réutilisable et complet

#### 🟡 `useWebSocket()`
```typescript
const useWebSocket = (packetID: string, callback: Listener) => {
  useEffect(() => {
    wsClient.on(packetID, callback);
    return () => wsClient.off(packetID, callback);
  }, [packetID, callback]);
};
```

**Verdict:** ✅ Bon - Wrapper propre autour du client WS

### 8. **WebSocket Client** (`app/websocket/websocket.tsx`)

**Implémentation:**
- Auto-reconnect après 5s de déconnexion
- Message queue si non connecté
- JSON parsing avec error handling
- Listeners pattern (Map<packetID, Listener[]>)

```typescript
class WebSocketClient {
  private socket: WebSocket | null = null;
  private listeners: Map<string, Listener[]> = new Map();
  private messageQueue: string[] = [];
}
```

**Verdict:** ⚠️ Passable - Fonctionne basiquement, mais superficiel
- Pas de heartbeat/ping
- Pas de retry backoff exponentiel
- Erreurs de parsing silencieuses (console.error seulement)

---

## 💪 Points Forts

### 1. **Architecture en Couches Claire**
- Séparation nette: Page → Client → Component → Pixi → Elements
- Chaque composant a une responsabilité unique
- Props bien typées (TypeScript strict)

### 2. **Hit Testing Impeccable**
- Géométrie bien pensée (cercle pour nodes, rectangle orienté pour edges)
- Logique mathématique correcte
- Pas de dépendance sur Pixi.js hit areas (évite bugs)

### 3. **Event Handling Robuste**
- Stage-level handlers évitent les bugs d'event propagation
- Refs pour éviter "stale closures"
- Manual hit testing à chaque click (flexible)

### 4. **Toast Notification System**
- Implémentation propre et réutilisable
- Auto-dismiss configurable
- Intégré aux opérations CRUD

### 5. **Keyboard Shortcuts**
- Support natif: E (edit), V (select), N (node), R (road)
- Delete/Backspace pour supprimer
- Esc pour déselectionner
- Bien documenté (_title attributes_)

### 6. **Optimisations de Performance**
- PointerPos conversion à ref pour réduire re-renders
- Conditional re-renders (setPointerPosState) uniquement si needed
- useCallback sur les event handlers importants

### 7. **TypeScript Coverage**
- Types explicites partout
- Interfaces bien définies (MapData, VehicleData, EditTool)
- Props interfaces complètes

### 8. **Code Lisibilité**
- Noms de variables clairs
- Commentaires aux endroits clés
- Structure logique facile à suivre

---

## ⚠️ Points Faibles & Améliorations

### 1. **Props Drilling - MAJEUR** 🔴

**Problème:**
```typescript
// MapPageClient
<MapComponent 
  editMode, setEditMode,
  activeTool, setActiveTool,
  selectedNodeId, setSelectedNodeId,
  selectedEdgeId, setSelectedEdgeId,
  onToast // 8+ props
/>
```

Traverse: MapComponent → PixiApp → MapCanvas  
Maintenance nightmare à l'ajout de nouvelles props

**Solution:**
```typescript
// Option 1: React Context
const MapEditorContext = createContext({
  editMode, setEditMode,
  activeTool, setActiveTool,
  // ...
});

// Option 2: Custom Hook
const useMapEditor = () => useContext(MapEditorContext);

// Usage:
const { activeTool, selectedNodeId } = useMapEditor();
```

**Effort:** Moyen | **Impact:** Haut

---

### 2. **Magic Numbers - MINEUR** 🟡

**Problème:**
```typescript
// MapCanvas.tsx
return dx*dx + dy*dy <= 16*16; // Pourquoi 16?
return distSq <= 7.5*7.5;      // Pourquoi 7.5?
```

**Solution:**
```typescript
// constants.ts
export const MAP_CONFIG = {
  NODE_RADIUS: 10,
  NODE_GLOW_RADIUS: 6,
  NODE_HIT_RADIUS: 16, // 10 + 6
  ROAD_WIDTH: 15,
  ROAD_HIT_RADIUS: 7.5, // width/2 + buffer?
};

// Usage:
return dx*dx + dy*dy <= MAP_CONFIG.NODE_HIT_RADIUS ** 2;
```

**Effort:** Minimal | **Impact:** Moyen (maintenabilité)

---

### 3. **Memory Potential Leaks - MODÉRÉ** 🟡

**Problème:**
```typescript
// MapCanvas.tsx - useEffect
app.stage.on('pointermove', onMove);
app.stage.on('pointerup', onUp);

// Cleanup?
return () => {
  app.stage.off('pointermove', onMove);
  app.stage.off('pointerup', onUp);
  // ... ✅ OK, mais dépendances correctes?
};
// deps: [editMode, draggingNodeId, dragPos, ...]
```

**Issue:** Dépendances nombreuses → fonction redéfinie souvent → new listeners → old listeners leak

**Solution:**
```typescript
// Utiliser un custom hook pour abstraction
const useStageEventHandler = (stage, handlers) => {
  useEffect(() => {
    Object.entries(handlers).forEach(([event, fn]) => {
      stage.on(event, fn);
    });
    return () => {
      Object.entries(handlers).forEach(([event, fn]) => {
        stage.off(event, fn);
      });
    };
  }, [stage, handlers]);
};
```

**Effort:** Moyen | **Impact:** Moyen

---

### 4. **No Input Validation - MAJEUR** 🔴

**Problème:**
```typescript
// MapCanvas onClick
if (tool === 'addNode') {
  sendPacket('addNode', { 
    x: Math.round(worldPos.x),  // No validation!
    y: Math.round(worldPos.y),  // Could be -999999?
    kind: 'Intersection',
    name: 'New Node'
  });
}
```

**Risques:**
- Nodes placés hors limites
- Noms dupliqués ou vides
- Lane counts négatifs
- Speed limits invalides

**Solution:**
```typescript
// validators.ts
export const validateNodePosition = (x: number, y: number, mapBounds: Bounds) => {
  if (x < mapBounds.minX || x > mapBounds.maxX) throw new Error('X out of bounds');
  if (y < mapBounds.minY || y > mapBounds.maxY) throw new Error('Y out of bounds');
};

export const validateNodeName = (name: string) => {
  if (!name.trim()) throw new Error('Name cannot be empty');
  if (name.length > 50) throw new Error('Name too long');
};

// Usage:
if (tool === 'addNode') {
  try {
    validateNodePosition(worldPos.x, worldPos.y, mapBounds);
    validateNodeName('New Node');
    sendPacket('addNode', {...});
  } catch (err) {
    onToast(err.message, 'error');
  }
}
```

**Effort:** Moyen | **Impact:** Haut (robustesse)

---

### 5. **Error Handling Shallow - MAJEUR** 🔴

**Problème:**
```typescript
// WebSocket - Errors silencieuses
socket.onerror = (error) => {
  console.error("[WebSocket] Error:", error);
  this.socket?.close();
  // User ne sait pas quoi faire!
};

// MapComponent - Data processing sans error catch
useWebSocket("mapEdit", (data: any) => {
  if (data.success) {
    setMapData(data.data);
  }
  // Quoi si data n'est pas du format attendu?
});
```

**Solution:**
```typescript
// wsClient
socket.onerror = (error) => {
  const message = error instanceof Error ? error.message : 'Connection error';
  this.notifyError?.(message); // Callback pour notifier UI
  console.error("[WebSocket] Error:", error);
};

// MapComponent
useWebSocket("mapEdit", (data: any) => {
  try {
    const parsedData = validateMapData(data); // Zod/Yup
    if (parsedData.success) {
      setMapData(parsedData.data);
    }
  } catch (err) {
    onToast('Failed to load map data', 'error');
    console.error(err);
  }
});
```

**Effort:** Moyen | **Impact:** Haut (UX)

---

### 6. **PropertiesPanel Auto-close - MINEUR** 🟡

**Problème:**
```typescript
// PropertiesPanel.tsx
return (
  <div className="absolute top-[15px] right-[15px] ...">
    {selectedNode && (
      <> {/* Si selectedNode = null, panel reste visible! */}
        ...
      </>
    )}
  </div>
);
```

Quand l'utilisateur clique quelque part pour déselectionner, le panel des props de l'ancienne sélection reste visible par intermittence.

**Solution:**
```typescript
return (
  selectedNode || selectedEdge ? (
    <div className="absolute top-[15px] right-[15px] ...">
      {selectedNode && <NodeProps />}
      {selectedEdge && <EdgeProps />}
    </div>
  ) : null
);
```

**Effort:** Minimal | **Impact:** Mineur (UX)

---

### 7. **Rubber-band Line Inefficient - MINEUR** 🟡

**Problème:**
```typescript
{editMode && activeTool === 'addRoad' && sourceNode && pointerPosRef.current && (
  <pixiGraphics
    draw={(g) => {
      g.clear();
      const src = getNodePos(sourceNode);
      const currentPos = pointerPosRef.current;
      g.setStrokeStyle({ color: 0xffff00, width: 2, alpha: 0.8 });
      g.moveTo(src.x, src.y);
      if (currentPos) {
        g.lineTo(currentPos.x, currentPos.y);
      }
      g.stroke();
    }}
  />
)}
```

À chaque frame du curseur, redraw complet. Pas efficace pour des maps énormes.

**Solution:**
```typescript
// Utiliser une primitive Line au lieu de Graphics
import { Line } from 'pixi.js';

const rubberbandRef = useRef<Line | null>(null);

useEffect(() => {
  if (!rubberbandRef.current) {
    rubberbandRef.current = new Line();
    stage.addChild(rubberbandRef.current);
  }

  const rubber = rubberbandRef.current;
  if (sourceNode && pointerPosRef.current) {
    rubber.updateLine(sourceNode.x, sourceNode.y, pointerPosRef.current.x, pointerPosRef.current.y);
    rubber.visible = true;
  } else {
    rubber.visible = false;
  }

  return () => {
    rubber.visible = false;
  };
}, [sourceNode]);
```

**Effort:** Moyen | **Impact:** Moyen (perf sur grosse carte)

---

### 8. **No Scaling/Bounds Handling - MAJEUR** 🔴

**Problème:**
Pas de gestion visible du "map boundaries". Utilisateur peut:
- Placer nodes partout (même hors limites)
- Zoomer/dézoomer arbitrairement
- Pas de limite de pan

**Solution:**
```typescript
// MapCanvas
const MAP_BOUNDS = { minX: 0, maxX: 10000, minY: 0, maxY: 10000 };

// Clamp worldPos
const clampPos = (pos) => ({
  x: Math.max(MAP_BOUNDS.minX, Math.min(MAP_BOUNDS.maxX, pos.x)),
  y: Math.max(MAP_BOUNDS.minY, Math.min(MAP_BOUNDS.maxY, pos.y)),
});

// Renderer: Show bounds (optional)
useEffect(() => {
  const boundary = new pixiGraphics();
  boundary.setStrokeStyle({ color: 0xff0000, width: 2, alpha: 0.3 });
  boundary.rect(MAP_BOUNDS.minX, MAP_BOUNDS.minY, ...);
  boundary.stroke();
  stage.addChild(boundary);
}, []);
```

**Effort:** Moyen | **Impact:** Haut (UX/robustesse)

---

### 9. **No Undo/Redo - MINEUR** 🟡

**Problème:**
Utilisateur ne peut pas annuler une action. Doit recharger la page pour reset.

**Solution future:**
```typescript
// mapStateHistory.ts
class MapStateHistory {
  private history: MapState[] = [];
  private currentIndex = -1;

  push(state: MapState) {
    this.history = this.history.slice(0, this.currentIndex + 1);
    this.history.push(state);
    this.currentIndex++;
  }

  undo() { if (this.currentIndex > 0) return this.history[--this.currentIndex]; }
  redo() { if (this.currentIndex < this.history.length - 1) return this.history[++this.currentIndex]; }
}
```

**Effort:** Élevé | **Impact:** Moyen (nice-to-have)

---

### 10. **CustomViewport Limited - MINEUR** 🟡

**Problème:**
```typescript
// CustomViewport.ts - Juste un wrapper autour Viewport
export class CustomViewport extends Viewport {
  constructor(options: IViewportOptions & {...}) {
    // Pas de logique spécifique
    const { decelerate, drag, pinch, wheel, ...rest } = options;
    super(rest);
    if (decelerate) this.decelerate();
    // ...
  }
}
```

C'est un simple proxy. Pourrait être natif Viewport.

**Action:** Supprimer ou ajouter logique vraiment custom (smooth pan limits, etc.)

**Effort:** Minimal | **Impact:** Minimal

---

## 📋 Recommandations Prioritaires

### Phase 1: CRITIQUE (Semaine 1)
1. **Input Validation** - Éviter nodes hors limites
2. **Error Handling** - Toast sur erreurs WebSocket
3. **Props Drilling** - Introduire Context API

**Effort estimé:** 8-12 heures

### Phase 2: IMPORTANT (Semaine 2)
4. **Magic Numbers** - Extraire vers constants.ts
5. **Memory Leaks Check** - Vérifier dépendances useEffect
6. **Bounds Visualization** - Montrer limites de la map

**Effort estimé:** 6-8 heures

### Phase 3: POLISH (Semaine 3)
7. **Rubber-band Line** - Optimiser rendering
8. **PropertiesPanel** - Auto-close quand rien selected
9. **Code Splitting** - Lazy load components lourds

**Effort estimé:** 6-10 heures

---

## 📈 Metrics de Qualité

| Aspect | Score | Notes |
|--------|-------|-------|
| Architecture | 8/10 | Couches claires, mais props drilling |
| Type Safety | 9/10 | TypeScript strict, bien typé |
| Error Handling | 5/10 | Basique, pas d'user feedback |
| Performance | 7/10 | Bien optimisé (refs), mais scalability? |
| Maintainability | 6/10 | Lisible, mais props drilling |
| Testing | 0/10 | Aucun test visible |
| Documentation | 4/10 | Minimal (quelques commentaires) |
| **Total** | **6.7/10** | **Bon, améliorable** |

---

## 🎯 Conclusion

### ✅ Ce qui Marche Bien
- Architecture en couches propre
- Hit testing impeccable
- Event handling robuste
- Toast notification system
- Keyboard shortcuts
- TypeScript typing

### ⚠️ Ce qui Marche Moins Bien
- **Props drilling excessive** → Context API needed
- **No validation/error handling** → Robustesse
- **Magic numbers** → Maintenance
- **Potential memory leaks** → Long sessions
- **No bounds checking** → Edge cases
- **Zero tests** → Regression risk

### 🚀 Prochaines Étapes
1. Introduire Context API ou Zustand
2. Ajouter validation sur tous les inputs
3. Améliorer error handling avec toast
4. Documenter le codebase
5. Ajouter tests unitaires/intégration

---

**Auteur:** Code Review Automtisé  
**Dernière mise à jour:** 26 Mars 2026
