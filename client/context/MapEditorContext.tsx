'use client';

import { createContext, useContext, ReactNode } from 'react';
import { EditTool } from '@/components/map/types';
import { ToastType } from '@/hooks/useToast';

/**
 * MapEditorContextType - Type définissant l'état global partagé
 * 
 * **Utilisation:**
 * Cet  interface décrit tous les propriétés et setters du contexte global
 * Utilisé par `useMapEditor()` hook pour accéder à l'état partout dans l'app
 * 
 * **Flux:**
 * ```
 * MapPageClient (root)
 *  └─ MapEditorProvider(state, handlers)
 *     ├─ Toolbar → useMapEditor() → activeTool, setActiveTool
 *     ├─ MapComponent → useMapEditor() → selectedNodeId, isSimulating
 *     ├─ PropertiesPanel → useMapEditor() → isSimulating, addToast
 *     └─ MapCanvas → useMapEditor() → activeTool, selectedNodeId, etc.
 * ```
 */
interface MapEditorContextType {
  // ============ OUTIL ACTIF ============
  // Quel outil est actuellement sélectionné (Pan, Select, AddNode, AddRoad)
  activeTool: EditTool;
  setActiveTool: (tool: EditTool) => void;
  
  // ============ SÉLECTION ============
  // Quel nœud est actuellement sélectionné (null = aucun)
  selectedNodeId: number | null;
  setSelectedNodeId: (id: number | null) => void;
  
  // Quelle route est actuellement sélectionnée (null = aucune)
  selectedEdgeId: number | null;
  setSelectedEdgeId: (id: number | null) => void;
  
  // ============ ÉTAT SIMULATION ============
  // Est-ce que la simulation est en cours? (true = véhicules se déplacent)
  isSimulating: boolean;
  setIsSimulating: (value: boolean) => void;
  
  // ============ NOTIFICATIONS ============
  // Affiche une toast notification (success, error, warning, info)
  addToast: (message: string, type: ToastType, duration?: number) => void;
}

const MapEditorContext = createContext<MapEditorContextType | undefined>(undefined);

/**
 * MapEditorProviderProps - Props pour initialiser le context provider
 * 
 * Tout l'état est passé EN PROPS du composant parent (MapPageClient)
 * pour éviter la duplication d'état. Le provider est juste un "tunnel"
 * qui fait transiter les valeurs à tous les enfants.
 * 
 * **Avantage:** L'état reste centralisé dans MapPageClient
 * **Inconvénient:** Le provider doit re-passer les props à chaque state change
 */
interface MapEditorProviderProps {
  children: ReactNode;
  activeTool: EditTool;
  setActiveTool: (tool: EditTool) => void;
  selectedNodeId: number | null;
  setSelectedNodeId: (id: number | null) => void;
  selectedEdgeId: number | null;
  setSelectedEdgeId: (id: number | null) => void;
  isSimulating: boolean;
  setIsSimulating: (value: boolean) => void;
  addToast: (message: string, type: ToastType, duration?: number) => void;
}

/**
 * MapEditorProvider - Context provider pour partager l'état global
 * 
 * **Responsabilités:**
 * 1. Crée un objet `value` contenant tous les props
 * 2. Passe ce `value` au MapEditorContext via <Context.Provider>
 * 3. Tous les enfants peuvent accéder `value` via `useMapEditor()`
 * 
 * **Architecture:**
 * ```
 * MapPageClient (gère l'état)
 *  └─ MapEditorProvider (partage l'état)
 *     └─ children
 *        ├─ Toolbar (utilise activeTool, setActiveTool)
 *        ├─ MapComponent (utilise selectedNodeId)
 *        ├─ PropertiesPanel (utilise isSimulating, addToast)
 *        └─ MapCanvas (utilise activeTool, selectedNodeId)
 * ```
 * 
 * **Pattern:** 
 * C'est un "pass-through" provider. L'état n'est pas créé ici,
 * il est créé dans MapPageClient et passé en props.
 * 
 * Cela évite la re-création du context à chaque re-render.
 */
export function MapEditorProvider({
  children,
  activeTool,
  setActiveTool,
  selectedNodeId,
  setSelectedNodeId,
  selectedEdgeId,
  setSelectedEdgeId,
  isSimulating,
  setIsSimulating,
  addToast,
}: MapEditorProviderProps) {
  // Assemble all props into a single value object
  const value: MapEditorContextType = {
    activeTool,
    setActiveTool,
    selectedNodeId,
    setSelectedNodeId,
    selectedEdgeId,
    setSelectedEdgeId,
    isSimulating,
    setIsSimulating,
    addToast,
  };

  // Partage le value à tous les enfants via le provider
  return (
    <MapEditorContext.Provider value={value}>
      {children}
    </MapEditorContext.Provider>
  );
}

/**
 * useMapEditor() - Hook pour accéder à l'état global de l'éditeur
 * 
 * **Responsabilités:**
 * - Retourne le contexte MapEditorContext
 * - Lance une erreur si utilisé en dehors de MapEditorProvider
 * - Permet à N'IMPORTE QUEL composant d'accéder/modifier l'état global
 * 
 * **Utilisation:**
 * ```typescript
 * // Dans n'importe quel composant (au sein de MapEditorProvider)
 * const { activeTool, setActiveTool, isSimulating, addToast } = useMapEditor();
 * 
 * // Peut être appelé plusieurs fois (retourne la même référence)
 * // Déclenche un re-render du composant quand l'état change
 * ```
 * 
 * **Composants qui l'utilisent:**
 * 1. **Toolbar** → activeTool, setActiveTool, isSimulating, setIsSimulating
 * 2. **MapComponent** → selectedNodeId, setSelectedNodeId, selectedEdgeId
 * 3. **PropertiesPanel** → isSimulating, addToast
 * 4. **MapCanvas** → activeTool, selectedNodeId, setSelectedNodeId, setSelectedEdgeId
 * 5. **PixiApp** → activeTool, selectedNodeId, isSimulating, addToast
 * 
 * @throws {Error} Si utilisé en dehors de MapEditorProvider
 * @returns {MapEditorContextType} L'état global et ses setters
 * 
 * @example
 * // Exemple d'utilisation
 * function MonComposant() {
 *   const { activeTool, setActiveTool } = useMapEditor();
 *   
 *   return (
 *     <button onClick={() => setActiveTool('pan')}>
 *       Outil actif: {activeTool}
 *     </button>
 *   );
 * }
 */
export function useMapEditor(): MapEditorContextType {
  const context = useContext(MapEditorContext);
  if (context === undefined) {
    throw new Error('useMapEditor must be used within MapEditorProvider');
  }
  return context;
}
