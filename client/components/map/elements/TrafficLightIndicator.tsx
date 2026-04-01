import { MapNode } from '../types';

export function TrafficLightIndicator({
	start,
	end,
	edgeId,
	isGreen,
}: {
	start: MapNode;
	end: MapNode;
	edgeId: number;
	isGreen: boolean;
}) {
	const width = 15;
	return (
		<pixiGraphics
			draw={(graphics) => {
				graphics.clear();
				const dx = end.x - start.x;
				const dy = end.y - start.y;
				const length = Math.sqrt(dx * dx + dy * dy);
				const angle = Math.atan2(dy, dx);

				graphics.position.set(start.x, start.y);
				graphics.rotation = angle;

				const color = isGreen ? 0x22c55e : 0xef4444;
				graphics.setStrokeStyle({ color, width: 4 });
				graphics.moveTo(length - 15, -width / 2);
				graphics.lineTo(length - 15, width / 2);
				graphics.stroke();
			}}
		/>
	);
}
