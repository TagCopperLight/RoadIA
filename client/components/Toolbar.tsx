'use client';

import { useState } from 'react';
import { useWs } from '@/app/websocket/websocket';
import { useEditMode, EditTool } from './EditModeContext';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

// Inline SVG icons

function IconSelect() {
    return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
            <path d="M4 0l16 12-7 2-4 8z" />
        </svg>
    );
}


function IconAddNode() {
    return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="9" />
            <line x1="12" y1="8" x2="12" y2="16" strokeLinecap="round" />
            <line x1="8" y1="12" x2="16" y2="12" strokeLinecap="round" />
        </svg>
    );
}

function IconAddRoad() {
    return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="5" cy="12" r="3" fill="currentColor" stroke="none" />
            <circle cx="19" cy="12" r="3" fill="currentColor" stroke="none" />
            <line x1="8" y1="12" x2="16" y2="12" strokeLinecap="round" />
        </svg>
    );
}

function IconPlay() {
    return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
            <polygon points="5,3 19,12 5,21" />
        </svg>
    );
}

function IconPause() {
    return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor">
            <rect x="5" y="4" width="4" height="16" rx="1" />
            <rect x="15" y="4" width="4" height="16" rx="1" />
        </svg>
    );
}

function IconReset() {
    return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M4 12a8 8 0 1 1 2 5.3" strokeLinecap="round" />
            <polyline points="4,7 4,12 9,12" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

function IconModeEdit() {
    return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" strokeLinecap="round" strokeLinejoin="round" />
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

function IconModeSimulation() {
    return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="10" />
            <polygon points="10,8 16,12 10,16" fill="currentColor" stroke="none" />
        </svg>
    );
}

function IconWaypoints() {
    return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="5" cy="7" r="2" fill="currentColor" />
            <circle cx="12" cy="12" r="2" fill="currentColor" />
            <circle cx="19" cy="17" r="2" fill="currentColor" />
            <path d="M7,9 L10,10 L14,14 L17,15" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

function IconIntersection() {
    return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="7" />
            <circle cx="12" cy="12" r="3" fill="currentColor" stroke="none" />
        </svg>
    );
}

function IconBus() {
    return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <rect x="2" y="5" width="20" height="12" rx="2" />
            <line x1="2" y1="10" x2="22" y2="10" />
            <line x1="8" y1="5" x2="8" y2="10" />
            <line x1="16" y1="5" x2="16" y2="10" />
            <circle cx="6.5" cy="19" r="1.5" fill="currentColor" stroke="none" />
            <circle cx="17.5" cy="19" r="1.5" fill="currentColor" stroke="none" />
        </svg>
    );
}

function IconDayNight({ dayNight }: { dayNight: boolean }) {
    return dayNight ? (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="5" />
            <line x1="12" y1="1" x2="12" y2="3" strokeLinecap="round" />
            <line x1="12" y1="21" x2="12" y2="23" strokeLinecap="round" />
            <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" strokeLinecap="round" />
            <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" strokeLinecap="round" />
            <line x1="1" y1="12" x2="3" y2="12" strokeLinecap="round" />
            <line x1="21" y1="12" x2="23" y2="12" strokeLinecap="round" />
            <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" strokeLinecap="round" />
            <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" strokeLinecap="round" />
        </svg>
    ) : (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor" stroke="none">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
        </svg>
    );
}

function IconDensity() {
    return (
        <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <rect x="3" y="10" width="18" height="4" rx="1" />
            <circle cx="7"  cy="12" r="1.5" fill="#22C55E" stroke="none" />
            <circle cx="12" cy="12" r="1.5" fill="#EAB308" stroke="none" />
            <circle cx="17" cy="12" r="1.5" fill="#EF4444" stroke="none" />
        </svg>
    );
}

function ToolButton({
    onClick,
    isSelected,
    disabled,
    title,
    children,
}: {
    onClick: () => void;
    isSelected?: boolean;
    disabled?: boolean;
    title: string;
    children: React.ReactNode;
}) {
    return (
        <button
            onClick={onClick}
            title={title}
            disabled={disabled}
            className={`flex items-center justify-center p-[10px] transition-opacity text-white
                ${disabled ? 'opacity-25 cursor-not-allowed' : isSelected ? 'opacity-75 cursor-pointer' : 'opacity-100 hover:opacity-50 cursor-pointer'}`}
        >
            {children}
        </button>
    );
}

function Separator() {
    return <div className="w-px h-[26px] bg-white opacity-20" />;
}

const SPEED_PRESETS = [1, 3, 5, 10, 30] as const;

export default function Toolbar({ uuid }: { uuid: string }) {
    const ws = useWs();
    const [speedMultiplier, setSpeedMultiplier] = useState(3);
    const {
        mode, editTool, simState,
        setMode, setEditTool, setSimState, setSelectedElement, setPendingRoadFrom, setSimulationResetAt, setShowScore,
        densityView, setDensityView, isDensityLoading, setIsDensityLoading,
        showIntersections, setShowIntersections,
        mapSettings, setMapSettings,
    } = useEditMode();

    const switchToEdit = () => {
        ws?.send('resetSimulation', {});
        setSimState('stopped');
        setShowScore(false);
        setDensityView(false);
        setIsDensityLoading(false);
        setSimulationResetAt(prev => prev + 1);
        setSelectedElement(null);
        setPendingRoadFrom(null);
        setMode('edit');
    };

    const handleDensityToggle = () => {
        if (densityView) {
            setDensityView(false);
        } else if (!isDensityLoading) {
            ws?.send('requestDensity', {});
            setIsDensityLoading(true);
        }
    };

    const switchToSimulation = () => {
        setSelectedElement(null);
        setPendingRoadFrom(null);
        setShowIntersections(true);
        setMode('simulation');
    };

    const handlePlayPause = () => {
        if (simState === 'running') {
            ws?.send('stopSimulation', {});
            setSimState('paused');
        } else {
            ws?.send('startSimulation', {});
            setSimState('running');
        }
    };

    const handleReset = () => {
        ws?.send('resetSimulation', {});
        setSimState('stopped');
        setShowScore(false);
        setSimulationResetAt(prev => prev + 1);
    };

    const handleDayNightToggle = async () => {
        if (!mapSettings || simState !== 'stopped') return;
        const updated = { ...mapSettings, use_day_night_cycle: !mapSettings.use_day_night_cycle };
        try {
            const res = await fetch(`${API_URL}/api/simulations/${uuid}/settings`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(updated),
            });
            if (!res.ok) return;
            setMapSettings(updated);
            ws?.send('resetSimulation', {});
            setSimState('stopped');
            setShowScore(false);
            setSimulationResetAt(prev => prev + 1);
        } catch {
            // network error — no-op
        }
    };

    const selectTool = (tool: EditTool) => {
        setEditTool(tool);
        setSelectedElement(null);
        setPendingRoadFrom(null);
    };

    return (
        <div className="flex items-center w-full pl-[15px] pr-[15px]">
            <div className="flex items-center justify-between bg-black rounded-[10px] w-full px-1">
                {/* Left: mode-specific items */}
                <div className="flex items-center">
                    {mode === 'edit' ? (
                        <>
                            <ToolButton onClick={() => selectTool('select')} isSelected={editTool === 'select'} title="Select">
                                <IconSelect />
                            </ToolButton>
                            <Separator />
                            <ToolButton onClick={() => selectTool('addNode')} isSelected={editTool === 'addNode'} title="Add Node">
                                <IconAddNode />
                            </ToolButton>
                            <Separator />
                            <ToolButton onClick={() => selectTool('addRoad')} isSelected={editTool === 'addRoad'} title="Add Road">
                                <IconAddRoad />
                            </ToolButton>
                            <Separator />
                            <ToolButton onClick={() => selectTool('waypoints')} isSelected={editTool === 'waypoints'} title="Waypoints">
                                <IconWaypoints />
                            </ToolButton>
                            <Separator />
                            <ToolButton onClick={() => selectTool('bus')} isSelected={editTool === 'bus'} title="Bus Lines">
                                <IconBus />
                            </ToolButton>
                        </>
                    ) : (
                        <>
                            <ToolButton onClick={handlePlayPause} title={simState === 'running' ? 'Pause' : 'Play'}>
                                {simState === 'running' ? <IconPause /> : <IconPlay />}
                            </ToolButton>
                            <Separator />
                            <ToolButton onClick={handleReset} title="Reset">
                                <IconReset />
                            </ToolButton>
                            <Separator />
                            {SPEED_PRESETS.map(speed => (
                                <button
                                    key={speed}
                                    onClick={() => {
                                        ws?.send('setSpeed', { multiplier: speed });
                                        setSpeedMultiplier(speed);
                                    }}
                                    title={`${speed}× speed`}
                                    className={`px-2 py-[10px] text-xs font-medium transition-opacity text-white cursor-pointer
                                        ${speedMultiplier === speed ? 'opacity-100' : 'opacity-30 hover:opacity-60'}`}
                                >
                                    {speed}×
                                </button>
                            ))}
                            <Separator />
                            <ToolButton
                                onClick={handleDensityToggle}
                                isSelected={densityView}
                                disabled={isDensityLoading}
                                title={isDensityLoading ? 'Computing...' : densityView ? 'Hide Density' : 'Density View'}
                            >
                                <IconDensity />
                            </ToolButton>
                            <Separator />
                            <ToolButton
                                onClick={() => setShowIntersections(!showIntersections)}
                                isSelected={!showIntersections}
                                title={showIntersections ? 'Hide Intersections' : 'Show Intersections'}
                            >
                                <IconIntersection />
                            </ToolButton>
                            <Separator />
                            <ToolButton
                                onClick={handleDayNightToggle}
                                isSelected={mapSettings ? !mapSettings.use_day_night_cycle : false}
                                disabled={simState !== 'stopped' || !mapSettings}
                                title={mapSettings?.use_day_night_cycle ? 'Day/Night Cycle (click for Immediate)' : 'Immediate Departure (click for Day/Night)'}
                            >
                                <IconDayNight dayNight={mapSettings?.use_day_night_cycle ?? true} />
                            </ToolButton>
                        </>
                    )}
                </div>

                {/* Right: mode toggle */}
                <ToolButton
                    onClick={mode === 'edit' ? switchToSimulation : switchToEdit}
                    title={mode === 'edit' ? 'Switch to Simulation Mode' : 'Switch to Edit Mode'}
                >
                    {mode === 'edit' ? <IconModeSimulation /> : <IconModeEdit />}
                </ToolButton>
            </div>
        </div>
    );
}
