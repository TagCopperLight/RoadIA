import { memo } from 'react';
import { MapNode } from '../types';

const LINK_TYPE_COLORS: Record<string, number> = {
	Priority: 0x22c55e,
	Yield: 0xf59e0b,
	Stop: 0xef4444,
	TrafficLight: 0x3b82f6,
};

interface IntersectionProps {
	node: MapNode;
	isSelected?: boolean;
	isEditMode?: boolean;
	isPendingFrom?: boolean;
	onSelect?: () => void;
	onAddRoad?: () => void;
}

export const Intersection = memo(function Intersection({
	node,
	isSelected,
	isEditMode,
	isPendingFrom,
	onSelect,
	onAddRoad,
}: IntersectionProps) {
	const isInteractive = isEditMode && (onSelect || onAddRoad);
	const handleTap = onSelect ?? onAddRoad;

	return (
		<pixiGraphics
			eventMode={isInteractive ? 'static' : 'none'}
			cursor={isInteractive ? 'pointer' : 'default'}
			onPointerTap={handleTap}
			draw={(g) => {
				g.clear();
				g.position.set(node.x, node.y);

			if (isSelected) {
				g.setFillStyle({ color: 0xfbbf24, alpha: 0.4 });
				g.circle(0, 0, node.radius + 4);
				g.fill();
			}

			g.setFillStyle({ color: 0x555555 });
			g.circle(0, 0, node.radius);
			g.fill();

				const ringColor = node.kind === 'Habitation' ? 0x3b82f6
					: node.kind === 'Workplace' ? 0xef4444
					: 0x888888;
				g.setStrokeStyle({ color: ringColor, width: 2 });
				g.circle(0, 0, node.radius);
				g.stroke();

			if (isPendingFrom) {
				g.setStrokeStyle({ color: 0x22c55e, width: 3 });
				g.circle(0, 0, node.radius + 4);
				g.stroke();
			}

				if (isSelected && node.internal_lanes && node.internal_lanes.length > 0) {
					for (const lane of node.internal_lanes) {
						const color = LINK_TYPE_COLORS[lane.link_type] ?? 0x22c55e;
						const ex = lane.entry[0] - node.x;
						const ey = lane.entry[1] - node.y;
						const exitX = lane.exit[0] - node.x;
						const exitY = lane.exit[1] - node.y;

						g.setStrokeStyle({ color, width: 0.5, alpha: 0.85 });
						g.moveTo(ex, ey);
						g.lineTo(exitX, exitY);
						g.stroke();


						const ddx = exitX - ex;
						const ddy = exitY - ey;
						const len = Math.sqrt(ddx * ddx + ddy * ddy);
						if (len > 0) {
							const ux = ddx / len;
							const uy = ddy / len;
							const px = -uy;
							const py = ux;
							const arrowLen = 2;
							const arrowHalf = 1;
							g.setFillStyle({ color, alpha: 0.85 });
							g.moveTo(exitX, exitY);
							g.lineTo(exitX - ux * arrowLen + px * arrowHalf, exitY - uy * arrowLen + py * arrowHalf);
							g.lineTo(exitX - ux * arrowLen - px * arrowHalf, exitY - uy * arrowLen - py * arrowHalf);
							g.fill();
						}
					}
				}
			}}
		/>
	);
});
