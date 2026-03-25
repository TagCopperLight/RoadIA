import { VehicleData } from '../types';

interface VehicleProps {
    data: VehicleData;
}

export function Vehicle({ data }: VehicleProps) {
    if (data.state === 'Arrived') {
        return null;
    }

    return (
        <pixiGraphics draw={(graphics) => {
            graphics.clear();
            
            let x = data.x;
            let y = data.y;

            if (data.heading !== undefined) {
                const offsetDistance = 4;
                const offsetX = offsetDistance * Math.cos(data.heading + Math.PI / 2);
                const offsetY = offsetDistance * Math.sin(data.heading + Math.PI / 2);
                x += offsetX;
                y += offsetY;
            }

            graphics.position.set(x, y);
            
            graphics.rotation = data.heading ?? 0;
            
            graphics.setFillStyle({ color: 'purple' });
            graphics.rect(-10, -3, 10, 6);
            graphics.fill();
        }} />
    );
}
