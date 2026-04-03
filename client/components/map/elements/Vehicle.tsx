import { useCallback } from 'react';
import { Graphics } from 'pixi.js';
import { VehicleData } from '../types';

interface VehicleProps {
    data: VehicleData;
}

export function Vehicle({ data }: VehicleProps) {
    const drawCar = useCallback((g: Graphics) => {
        g.clear();
        g.setFillStyle({ color: 'purple' });
        g.rect(-10, -2, 8, 5);
        g.fill();
    }, []);

    if (data.state === 'Arrived' || data.state === 'Waiting') {
        return null;
    }

    return (
        <pixiGraphics 
            x={data.x} 
            y={data.y} 
            rotation={data.heading ?? 0}
            draw={drawCar} 
        />
    );
}
