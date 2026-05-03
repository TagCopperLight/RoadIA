import { useCallback } from 'react';
import { Graphics } from 'pixi.js';
import { VehicleData } from '../types';

interface BusProps {
    data: VehicleData;
}

export function Bus({ data }: BusProps) {
    const drawBus = useCallback((g: Graphics) => {
        g.clear();
        g.setFillStyle({ color: 0x0066CC }); // Blue
        g.rect(-18, -4, 16, 8);
        g.fill();
    }, []);

    return (
        <pixiGraphics 
            x={data.x} 
            y={data.y} 
            rotation={data.heading ?? 0}
            draw={drawBus} 
            zIndex={100}
        />
    );
}
