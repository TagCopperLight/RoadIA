import { memo } from 'react';
import { FederatedPointerEvent } from 'pixi.js';
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
	isMovable?: boolean;
	onSelect?: () => void;
	onAddRoad?: () => void;
	onDragStart?: (id: number) => void;
	onDragCancel?: () => void;
}

export const Intersection = memo(function Intersection({
	node,
	isSelected,
	isEditMode,
	isPendingFrom,
	isMovable,
	onSelect,
	onAddRoad,
	onDragStart,
	onDragCancel,
}: IntersectionProps) {
	const isInteractive = isEditMode && (onSelect || onAddRoad || isMovable);
	const handleTap = onSelect ?? onAddRoad;

	const handlePointerDown = (e: FederatedPointerEvent) => {
		if (!isMovable || !onDragStart) return;
		e.stopPropagation(); // prevent viewport from starting a pan
		onDragStart(node.id);
	};

	// If the button is released while still over the intersection, cancel the drag
	const handlePointerUp = () => {
		onDragCancel?.();
	};

	return (
		<pixiGraphics
			eventMode={isInteractive ? 'static' : 'none'}
			cursor={isMovable ? 'grab' : isInteractive ? 'pointer' : 'default'}
			onPointerTap={isMovable ? undefined : handleTap}
			onPointerDown={isMovable ? handlePointerDown : undefined}
			onPointerUp={isMovable ? handlePointerUp : undefined}
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
						g.setStrokeStyle({ color, width: 2, alpha: 0.85 });
						g.moveTo(lane.entry[0] - node.x, lane.entry[1] - node.y);
						g.lineTo(lane.exit[0] - node.x, lane.exit[1] - node.y);
						g.stroke();
					}
				}
			}}
		/>
	);
});
