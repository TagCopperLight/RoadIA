import { FederatedPointerEvent } from 'pixi.js';
import { MapNode, EditTool } from '../types';
import { MAP_CONFIG } from '@/lib/constants';

interface IntersectionProps {
	node: MapNode;
	selected?: boolean;
	isAddRoadSource?: boolean;
	activeTool?: EditTool;
	isDragging?: boolean;
	onSelect?: (e: FederatedPointerEvent) => void;
	onDragStart?: (e: FederatedPointerEvent) => void;
}

export function Intersection({
	node,
	selected = false,
	isAddRoadSource = false,
	activeTool,
	isDragging = false,
	onSelect,
	onDragStart,
}: IntersectionProps) {
	const nodeColor = MAP_CONFIG.NODE_COLORS[node.kind as keyof typeof MAP_CONFIG.NODE_COLORS] || MAP_CONFIG.NODE_COLORS.Intersection;
	const radius = 10;
	const isInteractive = activeTool === 'select' || activeTool === 'addRoad';

	return (
		<pixiGraphics
			eventMode={isInteractive ? 'static' : 'none'}
			cursor={isInteractive ? 'pointer' : 'default'}
			alpha={isDragging ? 0.6 : 1}
			onClick={onSelect}
			onPointerDown={onDragStart}
			draw={(g) => {
				g.clear();
				g.position.set(node.x, node.y);

				// Selection / source glow ring
				if (selected || isAddRoadSource) {
					const glowColor = isAddRoadSource ? 0xffaa00 : 0xffff00;
					g.setFillStyle({ color: glowColor, alpha: 0.5 });
					g.circle(0, 0, radius + 6);
					g.fill();
				}

				// Main circle
				g.setFillStyle({ color: nodeColor });
				g.circle(0, 0, radius);
				g.fill();
			}}
		/>
	);
}
