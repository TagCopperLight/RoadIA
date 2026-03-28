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

            graphics.position.set(x, y);
            
            graphics.rotation = data.heading ?? 0;
            
            graphics.setFillStyle({ color: 'purple' });
            graphics.rect(-10, -3, 10, 6);
            graphics.fill();
        }} />
    );
}
