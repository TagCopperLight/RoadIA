'use client';

import Image from 'next/image';
import { useWs } from '@/app/websocket/websocket';

const TOOLS = [
    { icon: 'Move', alt: 'Move' },
    { icon: 'Edit', alt: 'Edit' },
    { icon: 'House', alt: 'House' },
    { icon: 'Building', alt: 'Building' },
    { icon: 'Play', alt: 'Play' },
    { icon: 'Octagon', alt: 'Stop' },
];

export default function Toolbar() {
    const ws = useWs();

    const handleToolClick = (tool: string) => {
        if (tool === 'Play') {
            ws?.send('startSimulation', {});
        } else if (tool === 'Stop') {
            ws?.send('resetSimulation', {});
        }
    };

    return (
        <div className="flex items-center w-full pl-[15px] pr-[15px]">
            <div className='flex items-center bg-black rounded-[10px] w-full'>
                {TOOLS.map((tool, index) => (
                    <div key={tool.alt} className="flex items-center" onClick={() => handleToolClick(tool.alt)}>
                        <Image
                            src={`/map/${tool.icon}.svg`}
                            alt={tool.alt}
                            width={24}
                            height={24}
                            className='m-[11px] cursor-pointer hover:opacity-50 transition-opacity'
                        />
                        {index < TOOLS.length - 1 && (
                            <Image src="/map/Separator.svg" alt="Separator" height={26} width={1} />
                        )}
                    </div>
                ))}
            </div>
        </div>
    )
}
