'use client';

import Image from 'next/image';
import { useEffect, useState } from 'react';
import { wsClient } from '@/app/websocket/websocket';
import { EditTool } from './map/types';

interface ToolbarProps {
    editMode: boolean;
    setEditMode: (v: boolean) => void;
    activeTool: EditTool;
    setActiveTool: (t: EditTool) => void;
    onClearSelection: () => void;
}

const EDIT_TOOLS: { icon: string; alt: string; tool: EditTool }[] = [
    { icon: 'Move', alt: 'Select', tool: 'select' },
    { icon: 'House', alt: 'Add Node', tool: 'addNode' },
    { icon: 'Edit', alt: 'Add Road', tool: 'addRoad' },
];

export default function Toolbar({ editMode, setEditMode, activeTool, setActiveTool, onClearSelection }: ToolbarProps) {
    const [isPlaying, setIsPlaying] = useState(false);

    const handlePlayPause = () => {
        if (isPlaying) {
            wsClient.send('stopSimulation', {});
            setIsPlaying(false);
        } else {
            wsClient.send('startSimulation', {});
            setIsPlaying(true);
        }
    };

    const handleReset = () => {
        wsClient.send('resetSimulation', {});
        setIsPlaying(false);
    };

    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;
            switch (e.key) {
                case 'e': case 'E':
                    setEditMode(!editMode);
                    onClearSelection();
                    break;
                case 'v': case 'V':
                    if (editMode) setActiveTool('select');
                    break;
                case 'n': case 'N':
                    if (editMode) setActiveTool('addNode');
                    break;
                case 'r': case 'R':
                    if (editMode) setActiveTool('addRoad');
                    break;
            }
        };
        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [editMode, setEditMode, setActiveTool, onClearSelection]);

    const toggleMode = () => {
        setEditMode(!editMode);
        onClearSelection();
    };

    return (
        <div className="flex items-center w-full pl-[15px] pr-[15px]">
            <div className='flex items-center bg-black rounded-[10px] w-full'>
                {editMode ? (
                    EDIT_TOOLS.map((tool, index) => (
                        <div key={tool.tool} className="flex items-center">
                            <div
                                onClick={() => setActiveTool(tool.tool)}
                                className={`flex items-center cursor-pointer transition-opacity ${activeTool === tool.tool ? 'opacity-100' : 'opacity-50 hover:opacity-75'}`}
                                title={tool.alt}
                            >
                                <Image src={`/map/${tool.icon}.svg`} alt={tool.alt} width={24} height={24} className='m-[11px]' />
                            </div>
                            {index < EDIT_TOOLS.length - 1 && (
                                <Image src="/map/Separator.svg" alt="Separator" height={26} width={1} />
                            )}
                        </div>
                    ))
                ) : (
                    <div className="flex items-center">
                        <div onClick={handlePlayPause} className="flex items-center cursor-pointer hover:opacity-50 transition-opacity" title={isPlaying ? 'Pause' : 'Play'}>
                            <Image src={isPlaying ? '/map/Pause.svg' : '/map/Play.svg'} alt={isPlaying ? 'Pause' : 'Play'} width={24} height={24} className='m-[11px]' />
                        </div>
                        <Image src="/map/Separator.svg" alt="Separator" height={26} width={1} />
                        <div onClick={handleReset} className="flex items-center cursor-pointer hover:opacity-50 transition-opacity" title="Reset">
                            <Image src="/map/Reset.svg" alt="Reset" width={24} height={24} className='m-[11px]' />
                        </div>
                    </div>
                )}

                <div className="flex items-center ml-auto">
                    <button
                        onClick={toggleMode}
                        className={`mx-[11px] px-[10px] py-[4px] rounded-[6px] text-[13px] font-medium transition-colors cursor-pointer bg-neutral-700 text-white hover:bg-neutral-600`}
                        title="Toggle Edit/Simulation mode (E)"
                    >
                        {editMode ? 'Simulation' : 'Edit'}
                    </button>
                </div>
            </div>
        </div>
    );
}
