'use client';

import { useWs } from '@/app/websocket/websocket';
import { useEditMode, EditTool } from './EditModeContext';

// Inline SVG icons

function IconSelect() {
    return (
        <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor">
            <path d="M4 0l16 12-7 2-4 8z" />
        </svg>
    );
}


function IconAddNode() {
    return (
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="9" />
            <line x1="12" y1="8" x2="12" y2="16" strokeLinecap="round" />
            <line x1="8" y1="12" x2="16" y2="12" strokeLinecap="round" />
        </svg>
    );
}

function IconAddRoad() {
    return (
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="5" cy="12" r="3" fill="currentColor" stroke="none" />
            <circle cx="19" cy="12" r="3" fill="currentColor" stroke="none" />
            <line x1="8" y1="12" x2="16" y2="12" strokeLinecap="round" strokeDasharray="2 2" />
        </svg>
    );
}

function IconPlay() {
    return (
        <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor">
            <polygon points="5,3 19,12 5,21" />
        </svg>
    );
}

function IconPause() {
    return (
        <svg width="28" height="28" viewBox="0 0 24 24" fill="currentColor">
            <rect x="5" y="4" width="4" height="16" rx="1" />
            <rect x="15" y="4" width="4" height="16" rx="1" />
        </svg>
    );
}

function IconReset() {
    return (
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M4 12a8 8 0 1 1 2 5.3" strokeLinecap="round" />
            <polyline points="4,7 4,12 9,12" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

function IconModeEdit() {
    return (
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7" strokeLinecap="round" strokeLinejoin="round" />
            <path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z" strokeLinecap="round" strokeLinejoin="round" />
        </svg>
    );
}

function IconModeSimulation() {
    return (
        <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
            <circle cx="12" cy="12" r="10" />
            <polygon points="10,8 16,12 10,16" fill="currentColor" stroke="none" />
        </svg>
    );
}

function ToolButton({
    onClick,
    isSelected,
    title,
    children,
}: {
    onClick: () => void;
    isSelected?: boolean;
    title: string;
    children: React.ReactNode;
}) {
    return (
        <button
            onClick={onClick}
            title={title}
            className={`flex items-center justify-center p-[16px] cursor-pointer transition-opacity text-white
                ${isSelected ? 'opacity-35' : 'opacity-100 hover:opacity-50'}`}
        >
            {children}
        </button>
    );
}

function Separator() {
    return <div className="w-px h-[40px] bg-white opacity-20" />;
}

export default function Toolbar() {
    const ws = useWs();
    const {
        mode, editTool, simState,
        setMode, setEditTool, setSimState, setSelectedElement, setPendingRoadFrom, setSimulationResetAt,
    } = useEditMode();

    const switchToEdit = () => {
        ws?.send('resetSimulation', {});
        setSimState('stopped');
        setSimulationResetAt(prev => prev + 1);
        setSelectedElement(null);
        setPendingRoadFrom(null);
        setMode('edit');
    };

    const switchToSimulation = () => {
        setSelectedElement(null);
        setPendingRoadFrom(null);
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
        setSimulationResetAt(prev => prev + 1);
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
