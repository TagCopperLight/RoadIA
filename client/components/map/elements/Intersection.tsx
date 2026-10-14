import { memo, useMemo } from 'react';
import { Graphics, Circle } from 'pixi.js';
import { MapNode } from '../types';
import { buildJunctionCorners, JunctionApproach, JunctionCorner } from '../intersectionGeometry';

const EMPTY_APPROACHES: JunctionApproach[] = [];

function traceSurface(g: Graphics, corners: JunctionCorner[]) {
	g.moveTo(corners[0].left.x, corners[0].left.y);
	for (const { right, control, nextLeft } of corners) {
		g.lineTo(right.x, right.y);
		g.quadraticCurveTo(control.x, control.y, nextLeft.x, nextLeft.y);
	}
	g.closePath();
}

const LINK_TYPE_COLORS: Record<string, number> = {
	Priority:     0x22c55e,  // green
	Yield:        0xf59e0b,  // amber
	Stop:         0xef4444,  // red
	TrafficLight: 0x3b82f6,  // blue
};

interface IntersectionProps {
	node: MapNode;
	approaches?: JunctionApproach[];
	corners?: JunctionCorner[];
	showMarker?: boolean;
	isSelected?: boolean;
	isEditMode?: boolean;
	isPendingFrom?: boolean;
	onSelect?: () => void;
	onAddRoad?: () => void;
	onWaypointClick?: () => void;
}

export const Intersection = memo(function Intersection({
	node,
	approaches = EMPTY_APPROACHES,
	corners: suppliedCorners,
	showMarker = true,
	isSelected,
	isEditMode,
	isPendingFrom,
	onSelect,
	onAddRoad,
	onWaypointClick,
}: IntersectionProps) {
	const corners = useMemo(() => suppliedCorners ?? buildJunctionCorners(node.radius, approaches), [suppliedCorners, node.radius, approaches]);
	const hitArea = useMemo(() => new Circle(0, 0, Math.max(node.radius, 6)), [node.radius]);
	const isInteractive = isEditMode && (onSelect || onAddRoad || onWaypointClick);
	const handleTap = onSelect ?? onAddRoad ?? onWaypointClick;

	return (
		<pixiGraphics
			eventMode={isInteractive ? 'static' : 'none'}
			hitArea={hitArea}
			cursor={isInteractive ? 'pointer' : 'default'}
			onPointerTap={handleTap}
			draw={(g) => {
				g.clear();
				g.position.set(node.x, node.y);

				// Cover road markings inside the junction with a continuous asphalt surface.
				if (corners.length) {
					traceSurface(g, corners);
					g.fill({ color: 0x555555 });

				} else {
					g.circle(0, 0, node.radius).fill({ color: 0x555555 });
				}

				if (isSelected || isPendingFrom) {
					if (corners.length) traceSurface(g, corners);
					else g.circle(0, 0, node.radius + 4);
					if (isSelected) g.fill({ color: 0xfbbf24, alpha: 0.25 });
					g.stroke({ color: isPendingFrom ? 0x22c55e : 0xfbbf24, width: 3 });
				}

				// Small editor/type marker, independent of the generated pavement.
				if (showMarker) {
					const color = node.kind === 'Habitation' ? 0x3b82f6
						: node.kind === 'Workplace' ? 0xef4444 : 0x888888;
					g.circle(0, 0, Math.min(node.radius, 2.5)).fill({ color });
				}

				// Internal lanes drawn on top of intersection body
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

						// Arrowhead at exit point
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
