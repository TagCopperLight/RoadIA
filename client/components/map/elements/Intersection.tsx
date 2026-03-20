import { FederatedPointerEvent } from 'pixi.js';
import { MapNode, EditTool } from '../types';

interface IntersectionProps {
	node: MapNode;
	selected?: boolean;
	isAddRoadSource?: boolean;
	editMode?: boolean;
	activeTool?: EditTool;
	isDragging?: boolean;
	onSelect?: (e: FederatedPointerEvent) => void;
	onDragStart?: (e: FederatedPointerEvent) => void;
}

export function Intersection({
	node,
	selected = false,
	isAddRoadSource = false,
	editMode = false,
	activeTool,
	isDragging = false,
	onSelect,
	onDragStart,
}: IntersectionProps) {
	const nodeColor = node.kind === 'Habitation' ? 0x3366ff : node.kind === 'Workplace' ? 0xff3333 : 0xaaaaaa;
	const radius = 10;
	const isInteractive = editMode && (activeTool === 'select' || activeTool === 'addRoad');

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
