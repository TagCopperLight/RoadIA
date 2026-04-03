import { useCallback } from 'react';
import { VehicleData } from '../types';

interface VehicleProps {
    data: VehicleData;
}

export function Vehicle({ data }: VehicleProps) {
    if (data.state === 'Arrived' || data.state === 'Waiting') {
        return null;
    }

    const drawCar = useCallback((g: any) => {
        g.clear();
        g.setFillStyle({ color: 'purple' });
        g.rect(-10, -2, 8, 5);
        g.fill();
    }, []);

    return (
        <pixiGraphics 
            x={data.x} 
            y={data.y} 
            rotation={data.heading ?? 0}
            draw={drawCar} 
        />
    );
}
