import { MapNode, MapEdge } from '../types';

export function TrafficLightIndicator({
	start,
	end,
	edge,
	isGreen,
}: {
	start: MapNode;
	end: MapNode;
	edge: MapEdge;
	isGreen: boolean;
}) {
	return (
		<pixiGraphics
			draw={(g) => {
				g.clear();
				const dx = end.x - start.x;
				const dy = end.y - start.y;
				const length = Math.sqrt(dx * dx + dy * dy);
				const angle = Math.atan2(dy, dx);

				g.position.set(start.x, start.y);
				g.rotation = angle;

				const fwWidth = edge.lane_count * edge.lane_width;
				const stopX = length - end.radius - 4;
				const color = isGreen ? 0x22c55e : 0xef4444;

				// Stop line across the forward lanes
				g.setStrokeStyle({ color, width: 2 });
				g.moveTo(stopX, 0);
				g.lineTo(stopX, fwWidth);
				g.stroke();

				// Signal dot on the outer road edge
				g.setFillStyle({ color });
				g.circle(stopX, fwWidth + 6, 4);
				g.fill();
			}}
		/>
	);
}
