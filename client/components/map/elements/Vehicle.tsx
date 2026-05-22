import { useCallback } from 'react';
import { Graphics } from 'pixi.js';
import { VehicleData, Motorization } from '../types';

interface VehicleProps {
    data: VehicleData;
}

const MOTOR_STYLE: Record<Motorization, { color: number; w: number; h: number }> = {
    Hybride: { color: 0xA855F7, w: 10, h: 5 },
    Electrique: { color: 0x06B6D4, w: 8, h: 4 },
    Essence: { color: 0xF59E0B, w: 10, h: 5 },
    Diesel: { color: 0x8B7355, w: 10, h: 5 },
};

const BUS_STYLE = { color: 0x22C55E, w: 18, h: 5 };

export function Vehicle({ data }: VehicleProps) {
    const style = data.kind === 'Bus'
        ? BUS_STYLE
        : data.motorization ? MOTOR_STYLE[data.motorization] : MOTOR_STYLE.Essence;

    const drawCar = useCallback((g: Graphics) => {
        g.clear();
        g.setFillStyle({ color: style.color });
        g.rect(-style.w / 2, -style.h / 2, style.w, style.h);
        g.fill();
    }, [style.color, style.w, style.h]);

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
