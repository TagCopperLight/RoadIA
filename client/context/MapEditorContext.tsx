'use client';

import { createContext, useContext, ReactNode } from 'react';
import { EditTool } from '@/components/map/types';
import { ToastType } from '@/hooks/useToast';

interface MapEditorContextType {
  // Tool selection
  activeTool: EditTool;
  setActiveTool: (tool: EditTool) => void;
  
  // Selection
  selectedNodeId: number | null;
  setSelectedNodeId: (id: number | null) => void;
  
  selectedEdgeId: number | null;
  setSelectedEdgeId: (id: number | null) => void;
  
  // Notifications
  addToast: (message: string, type: ToastType, duration?: number) => void;
}

const MapEditorContext = createContext<MapEditorContextType | undefined>(undefined);

interface MapEditorProviderProps {
  children: ReactNode;
  activeTool: EditTool;
  setActiveTool: (tool: EditTool) => void;
  selectedNodeId: number | null;
  setSelectedNodeId: (id: number | null) => void;
  selectedEdgeId: number | null;
  setSelectedEdgeId: (id: number | null) => void;
  addToast: (message: string, type: ToastType, duration?: number) => void;
}

export function MapEditorProvider({
  children,
  activeTool,
  setActiveTool,
  selectedNodeId,
  setSelectedNodeId,
  selectedEdgeId,
  setSelectedEdgeId,
  addToast,
}: MapEditorProviderProps) {
  const value: MapEditorContextType = {
    activeTool,
    setActiveTool,
    selectedNodeId,
    setSelectedNodeId,
    selectedEdgeId,
    setSelectedEdgeId,
    addToast,
  };

  return (
    <MapEditorContext.Provider value={value}>
      {children}
    </MapEditorContext.Provider>
  );
}

/**
 * Hook to access map editor context
 * Throws error if used outside of MapEditorProvider
 */
export function useMapEditor(): MapEditorContextType {
  const context = useContext(MapEditorContext);
  if (context === undefined) {
    throw new Error('useMapEditor must be used within MapEditorProvider');
  }
  return context;
}
