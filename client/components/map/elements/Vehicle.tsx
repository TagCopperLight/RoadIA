import { VehicleData } from '../types';

interface VehicleProps {
    data: VehicleData;
}

export function Vehicle({ data }: VehicleProps) {
    return (
        <pixiGraphics draw={(graphics) => {
            graphics.clear();
            graphics.position.set(data.x, data.y);
            graphics.setFillStyle({ color: 'purple' });
            graphics.circle(0, 0, 5);
            graphics.fill();
        }} />
    );
}
