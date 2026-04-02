import { VehicleData } from '../types';

interface VehicleProps {
    data: VehicleData;
}

export function Vehicle({ data }: VehicleProps) {
    if (data.state === 'Arrived' || data.state === 'Waiting') {
        return null;
    }

    return (
        <pixiGraphics draw={(graphics) => {
            graphics.clear();
            
            const x = data.x;
            const y = data.y;

            graphics.position.set(x, y);
            
            graphics.rotation = data.heading ?? 0;
            
            graphics.setFillStyle({ color: 'purple' });
            graphics.rect(-10, -2, 8, 5);
            graphics.fill();
        }} />
    );
}
