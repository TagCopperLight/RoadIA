'use client';

import Image from 'next/image';
import { useEffect, useState } from 'react';
import { wsClient } from '@/app/websocket/websocket';
import { useMapEditor } from '@/context/MapEditorContext';

const EDIT_TOOLS: { icon: string; alt: string; tool: string }[] = [
    { icon: 'Hand', alt: 'Pan', tool: 'pan' },
    { icon: 'Move', alt: 'Select', tool: 'select' },
    { icon: 'House', alt: 'Add Node', tool: 'addNode' },
    { icon: 'Edit', alt: 'Add Road', tool: 'addRoad' },
];

export default function Toolbar() {
    const { activeTool, setActiveTool } = useMapEditor();
    const [isPlaying, setIsPlaying] = useState(false);

    const handlePlayPause = () => {
        if (isPlaying) {
            wsClient.send('stopSimulation', {});
            setIsPlaying(false);
        } else {
            wsClient.send('startSimulation', {});
            setActiveTool('pan');
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
                case 'm': case 'M':
                    setActiveTool('pan');
                    break;
                case 'v': case 'V':
                    setActiveTool('select');
                    break;
                case 'n': case 'N':
                    setActiveTool('addNode');
                    break;
                case 'r': case 'R':
                    setActiveTool('addRoad');
                    break;
            }
        };
        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [setActiveTool]);

    return (
        <div className="flex items-center w-full pl-[15px] pr-[15px]">
            <div className='flex items-center bg-black rounded-[10px] w-full'>
                {/* Edit Tools */}
                {EDIT_TOOLS.map((tool, index) => (
                    <div key={tool.tool} className="flex items-center">
                        <div
                            onClick={() => setActiveTool(tool.tool as any)}
                            className={`flex items-center cursor-pointer transition-opacity ${activeTool === tool.tool ? 'opacity-100' : 'opacity-50 hover:opacity-75'}`}
                            title={tool.alt}
                        >
                            <Image src={`/map/${tool.icon}.svg`} alt={tool.alt} width={24} height={24} className='m-[11px]' />
                        </div>
                        {index < EDIT_TOOLS.length - 1 && (
                            <Image src="/map/Separator.svg" alt="Separator" height={26} width={1} />
                        )}
                    </div>
                ))}

                {/* Separator between Edit and Simulation tools */}
                <Image src="/map/Separator.svg" alt="Separator" height={26} width={1} className='mx-[4px]' />

                {/* Simulation Tools */}
                <div onClick={handlePlayPause} className="flex items-center cursor-pointer hover:opacity-50 transition-opacity" title={isPlaying ? 'Pause' : 'Play'}>
                    <Image src={isPlaying ? '/map/Pause.svg' : '/map/Play.svg'} alt={isPlaying ? 'Pause' : 'Play'} width={24} height={24} className='m-[11px]' />
                </div>
                <Image src="/map/Separator.svg" alt="Separator" height={26} width={1} />
                <div onClick={handleReset} className="flex items-center cursor-pointer hover:opacity-50 transition-opacity" title="Reset">
                    <Image src="/map/Reset.svg" alt="Reset" width={24} height={24} className='m-[11px]' />
                </div>
            </div>
        </div>
    );
}
