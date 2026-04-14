import { useCallback } from 'react';
import { Graphics } from 'pixi.js';
import { VehicleData } from '../types';

interface VehicleProps {
    data: VehicleData;
}

export function Vehicle({ data }: VehicleProps) {
    const drawCar = useCallback((g: Graphics) => {
        g.clear();
        
        // Get size and color based on vehicle type (motorization)
        // CITEPA/EPA/SDES sources: https://www.citepa.org/, https://www.epa.gov/
        const motorization = data.motorization || 'EssenceHybride';
        let width = 8.0;   // default
        let height = 5.0;  // default
        let color = 0xA855F7; // default purple (Essence Hybrid)
        
        switch (motorization) {
            case 'Electrique':
                width = 8.0;
                height = 4.0;
                color = 0x06B6D4;  // Cyan (electric)
                break;
            case 'Hybride':
                width = 10.0;
                height = 5.0;
                color = 0xA855F7;  // Violet (hybrid)
                break;
            case 'Essence':
                width = 10.0;
                height = 5.0;
                color = 0xF59E0B;  // Amber (essence)
                break;
            case 'Diesel':
                width = 10.0;
                height = 5.0;
                color = 0x8B7355;  // Brown (diesel)
                break;
        }
        
        g.setFillStyle({ color });
        g.rect(-width / 2, -height / 2, width, height);
        g.fill();
    }, [data.motorization]);

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
