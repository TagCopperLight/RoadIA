import { useCallback } from 'react';
import { Graphics } from 'pixi.js';
import { VehicleData } from '../types';

interface VehicleProps {
    data: VehicleData;
}

export function Vehicle({ data }: VehicleProps) {
    const drawVehicle = useCallback((g: Graphics) => {
        g.clear();
        
        // Determine if it's a bus or car
        const isBus = data.kind === 'Bus';
        
        if (isBus) {
            // Draw bus - larger and green
            const width = 12.0;
            const height = 6.0;
            const color = 0x22c55e; // Green
            
            g.setFillStyle({ color });
            g.rect(-width / 2, -height / 2, width, height);
            g.fill();

        } else {
            // Draw car - smaller and purple
            const [width, height] = data.motorization === 'Electrique' ? [8.0, 4.0] : [10.0, 5.0];
            const color = 0xA855F7;
            
            g.setFillStyle({ color });
            g.rect(-width / 2, -height / 2, width, height);
            g.fill();
        }
    }, [data.motorization, data.kind]);

    if (data.state === 'Arrived' || data.state === 'Waiting') {
        return null;
    }

    return (
        <pixiGraphics 
            x={data.x} 
            y={data.y} 
            rotation={data.heading ?? 0}
            draw={drawVehicle} 
        />
    );
}
