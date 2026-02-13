import { VehicleData } from '../types';

interface VehicleProps {
    data: VehicleData;
}

export function Vehicle({ data }: VehicleProps) {
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
            graphics.setFillStyle({ color: 'purple' });
            graphics.circle(0, 0, 5);
            graphics.fill();
        }} />
    );
}
