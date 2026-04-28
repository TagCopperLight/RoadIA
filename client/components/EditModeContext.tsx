'use client';

import { createContext, useContext, useState, ReactNode, Dispatch, SetStateAction } from 'react';

export type AppMode = "edit" | "simulation";
export type EditTool = "select" | "addNode" | "addRoad" | "waypoints";
export type SimState = "stopped" | "running" | "paused";
export type SelectedElement =
    | { type: "node"; id: number }
    | { type: "road"; canonicalId: number; reverseId?: number }
    | null;

interface EditModeContextType {
    mode: AppMode;
    editTool: EditTool;
    simState: SimState;
    selectedElement: SelectedElement;
    pendingRoadFrom: number | null;
    simulationResetAt: number;
    setMode: (m: AppMode) => void;
    setEditTool: (t: EditTool) => void;
    setSimState: (s: SimState) => void;
    setSelectedElement: (e: SelectedElement) => void;
    setPendingRoadFrom: (id: number | null) => void;
    setSimulationResetAt: Dispatch<SetStateAction<number>>;
}

const EditModeContext = createContext<EditModeContextType | null>(null);

export function EditModeProvider({ children }: { children: ReactNode }) {
    const [mode, setMode] = useState<AppMode>("edit");
    const [editTool, setEditTool] = useState<EditTool>("select");
    const [simState, setSimState] = useState<SimState>("stopped");
    const [selectedElement, setSelectedElement] = useState<SelectedElement>(null);
    const [pendingRoadFrom, setPendingRoadFrom] = useState<number | null>(null);
    const [simulationResetAt, setSimulationResetAt] = useState(0);

    return (
        <EditModeContext.Provider value={{
            mode, editTool, simState, selectedElement, pendingRoadFrom, simulationResetAt,
            setMode, setEditTool, setSimState, setSelectedElement, setPendingRoadFrom, setSimulationResetAt,
        }}>
            {children}
        </EditModeContext.Provider>
    );
}

export function useEditMode() {
    const ctx = useContext(EditModeContext);
    if (!ctx) throw new Error("useEditMode must be used within EditModeProvider");
    return ctx;
}
