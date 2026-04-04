import { memo } from 'react';
import { MapNode } from '../types';

const LINK_TYPE_COLORS: Record<string, number> = {
	Priority:     0x22c55e,  // green
	Yield:        0xf59e0b,  // amber
	Stop:         0xef4444,  // red
	TrafficLight: 0x3b82f6,  // blue
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

				// Selection highlight: amber transparent fill behind intersection
				if (isSelected) {
					g.setFillStyle({ color: 0xfbbf24, alpha: 0.4 });
					g.circle(0, 0, node.radius + 6);
					g.fill();
				}

				// Intersection body
				g.setFillStyle({ color: 0x555555 });
				g.circle(0, 0, node.radius);
				g.fill();

				// Kind ring
				const ringColor = node.kind === 'Habitation' ? 0x3b82f6
					: node.kind === 'Workplace' ? 0xef4444
					: 0x888888;
				g.setStrokeStyle({ color: ringColor, width: 2 });
				g.circle(0, 0, node.radius);
				g.stroke();

				// Pending-from indicator (first node in addRoad): green outer ring
				if (isPendingFrom) {
					g.setStrokeStyle({ color: 0x22c55e, width: 3 });
					g.circle(0, 0, node.radius + 4);
					g.stroke();
				}

				// Internal lanes drawn on top of intersection body
				if (isSelected && node.internal_lanes && node.internal_lanes.length > 0) {
					for (const lane of node.internal_lanes) {
						const color = LINK_TYPE_COLORS[lane.link_type] ?? 0x22c55e;
						g.setStrokeStyle({ color, width: 1, alpha: 0.85 });
						g.moveTo(lane.entry[0] - node.x, lane.entry[1] - node.y);
						g.lineTo(lane.exit[0] - node.x, lane.exit[1] - node.y);
						g.stroke();
					}
				}
			}}
		/>
	);
});
