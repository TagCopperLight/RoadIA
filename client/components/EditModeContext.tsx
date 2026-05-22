'use client';

import { createContext, useContext, useState, ReactNode, Dispatch, SetStateAction } from 'react';
<<<<<<< HEAD

export interface MapSettings {
    vehicle_count: number;
    simulation_duration: number;
=======
import { BusLine } from './map/types';

export interface MapSettings {
    vehicle_count: number;
    simulation_start_time: number;
    time_step: number;
>>>>>>> origin/dev
    max_budget: number;
    base_cost_per_meter: number;
    intersection_cost: number;
    habitation_cost: number;
    workplace_cost: number;
    time_weight: number;
    success_weight: number;
    pollution_weight: number;
    infrastructure_weight: number;
<<<<<<< HEAD
}

export type AppMode = "edit" | "simulation";
export type EditTool = "select" | "addNode" | "addRoad" | "waypoints";
=======
    use_day_night_cycle: boolean;
}

export type AppMode = "edit" | "simulation";
export type EditTool = "select" | "addNode" | "addRoad" | "waypoints" | "bus";
>>>>>>> origin/dev
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
    showScore: boolean;
    setShowScore: (show: boolean) => void;
    isScoringLoading: boolean;
    setIsScoringLoading: (loading: boolean) => void;
<<<<<<< HEAD
=======
    scoreProgress: number | null;
    setScoreProgress: Dispatch<SetStateAction<number | null>>;
>>>>>>> origin/dev
    densityView: boolean;
    setDensityView: (v: boolean) => void;
    isDensityLoading: boolean;
    setIsDensityLoading: (v: boolean) => void;
    showIntersections: boolean;
    setShowIntersections: (v: boolean) => void;
    showSettings: boolean;
    setShowSettings: (v: boolean) => void;
    mapSettings: MapSettings | null;
    setMapSettings: (s: MapSettings) => void;
    // Waypoint state
    waypointClickedNodeId: number | null;
    setWaypointClickedNodeId: (id: number | null) => void;
    waypointVehicleId: number | null;
    setWaypointVehicleId: (id: number | null) => void;
    pendingWaypoints: number[];
    setPendingWaypoints: Dispatch<SetStateAction<number[]>>;
<<<<<<< HEAD
=======
    // Bus line state
    busLines: BusLine[];
    setBusLines: Dispatch<SetStateAction<BusLine[]>>;
    busPendingStops: number[];
    setBusPendingStops: Dispatch<SetStateAction<number[]>>;
    busLineName: string;
    setBusLineName: (n: string) => void;
    busCreating: boolean;
    setBusCreating: (v: boolean) => void;
>>>>>>> origin/dev
}

const EditModeContext = createContext<EditModeContextType | null>(null);

export function EditModeProvider({ children }: { children: ReactNode }) {
    const [mode, setMode] = useState<AppMode>("edit");
    const [editTool, setEditTool] = useState<EditTool>("select");
    const [simState, setSimState] = useState<SimState>("stopped");
    const [selectedElement, setSelectedElement] = useState<SelectedElement>(null);
    const [pendingRoadFrom, setPendingRoadFrom] = useState<number | null>(null);
    const [simulationResetAt, setSimulationResetAt] = useState(0);
    const [showScore, setShowScore] = useState(false);
    const [isScoringLoading, setIsScoringLoading] = useState(false);
<<<<<<< HEAD
=======
    const [scoreProgress, setScoreProgress] = useState<number | null>(null);
>>>>>>> origin/dev
    const [densityView, setDensityView] = useState(false);
    const [isDensityLoading, setIsDensityLoading] = useState(false);
    const [showIntersections, setShowIntersections] = useState(true);
    const [showSettings, setShowSettings] = useState(false);
    const [mapSettings, setMapSettings] = useState<MapSettings | null>(null);
    const [waypointClickedNodeId, setWaypointClickedNodeId] = useState<number | null>(null);
    const [waypointVehicleId, setWaypointVehicleId] = useState<number | null>(null);
    const [pendingWaypoints, setPendingWaypoints] = useState<number[]>([]);
<<<<<<< HEAD
=======
    const [busLines, setBusLines] = useState<BusLine[]>([]);
    const [busPendingStops, setBusPendingStops] = useState<number[]>([]);
    const [busLineName, setBusLineName] = useState('');
    const [busCreating, setBusCreating] = useState(false);
>>>>>>> origin/dev

    return (
        <EditModeContext.Provider value={{
            mode, editTool, simState, selectedElement, pendingRoadFrom, simulationResetAt,
            showScore, setShowScore, isScoringLoading, setIsScoringLoading,
<<<<<<< HEAD
=======
            scoreProgress, setScoreProgress,
>>>>>>> origin/dev
            densityView, setDensityView, isDensityLoading, setIsDensityLoading,
            showIntersections, setShowIntersections,
            showSettings, setShowSettings,
            mapSettings, setMapSettings,
            setMode, setEditTool, setSimState, setSelectedElement, setPendingRoadFrom, setSimulationResetAt,
            waypointClickedNodeId, setWaypointClickedNodeId,
            waypointVehicleId, setWaypointVehicleId,
            pendingWaypoints, setPendingWaypoints,
<<<<<<< HEAD
=======
            busLines, setBusLines,
            busPendingStops, setBusPendingStops,
            busLineName, setBusLineName,
            busCreating, setBusCreating,
>>>>>>> origin/dev
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
