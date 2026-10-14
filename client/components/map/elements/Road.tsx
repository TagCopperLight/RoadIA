import { memo } from 'react';
import { Graphics } from 'pixi.js';
import { MapNode, MapEdge } from '../types';
import { junctionReach } from '../intersectionGeometry';

interface RoadProps {
	canonicalEdge: MapEdge;  // startNode → endNode; lanes on positive y (right of travel)
	reverseEdge?: MapEdge;   // endNode → startNode; lanes on negative y (two-way only)
	startNode: MapNode;
	endNode: MapNode;
	startReach?: number;
	endReach?: number;
	isSelected?: boolean;
	isEditMode?: boolean;
	onSelect?: () => void;
	densityView?: boolean;
	speedRatio?: number;
	reverseSpeedRatio?: number;
}

function speedRatioToColor(ratio: number): number {
	// ratio=0 (stuck): #EF4444 red  →  ratio=1 (flowing): #22C55E green
	const r = Math.round(0xEF + (0x22 - 0xEF) * ratio);
	const g = Math.round(0x44 + (0xC5 - 0x44) * ratio);
	const b = Math.round(0x44 + (0x5E - 0x44) * ratio);
	return (r << 16) | (g << 8) | b;
}

function drawDashedLine(
	g: Graphics,
	x1: number, y1: number,
	x2: number, y2: number,
	color: number,
	width: number,
	dashLen = 8,
	gapLen = 8,
) {
	const dx = x2 - x1;
	const dy = y2 - y1;
	const totalLen = Math.sqrt(dx * dx + dy * dy);
	const ux = dx / totalLen;
	const uy = dy / totalLen;

	g.setStrokeStyle({ color, width });
	let dist = 0;
	let drawing = true;
	while (dist < totalLen) {
		const segLen = Math.min(drawing ? dashLen : gapLen, totalLen - dist);
		if (drawing) {
			g.moveTo(x1 + ux * dist, y1 + uy * dist);
			g.lineTo(x1 + ux * (dist + segLen), y1 + uy * (dist + segLen));
			g.stroke();
		}
		dist += segLen;
		drawing = !drawing;
	}
}

export const Road = memo(function Road({ canonicalEdge, reverseEdge, startNode, endNode, startReach, endReach, isSelected, isEditMode, onSelect, densityView, speedRatio, reverseSpeedRatio }: RoadProps) {
	return (
		<pixiGraphics
			eventMode={isEditMode && onSelect ? 'static' : 'none'}
			cursor={isEditMode && onSelect ? 'pointer' : 'default'}
			onPointerTap={onSelect}
			draw={(g) => {
				g.clear();

				const dx = endNode.x - startNode.x;
				const dy = endNode.y - startNode.y;
				const length = Math.sqrt(dx * dx + dy * dy);
				const angle = Math.atan2(dy, dx);

				g.position.set(startNode.x, startNode.y);
				g.rotation = angle;

				const laneWidth = canonicalEdge.lane_width;
				const isTwoWay = !!reverseEdge;
				const fwCount = canonicalEdge.lane_count;
				const bwCount = reverseEdge?.lane_count ?? 0;
				const fwWidth = fwCount * laneWidth;
				const bwLaneWidth = reverseEdge?.lane_width ?? laneWidth;
				const bwWidth = bwCount * bwLaneWidth;

				const from = startReach ?? junctionReach(startNode.radius, length, bwWidth, fwWidth);
				const to = length - (endReach ?? junctionReach(endNode.radius, length, fwWidth, bwWidth));
				const visibleLength = to - from;
				if (visibleLength <= 0.001) return;

				// Selection highlight behind road
				if (isSelected) {
					g.setFillStyle({ color: 0xfbbf24, alpha: 0.4 });
					g.rect(from - 4, -bwWidth - 4, visibleLength + 8, fwWidth + bwWidth + 8);
					g.fill();
				}

				// Road surface
				if (densityView && isTwoWay) {
					const fwColor = speedRatio !== undefined ? speedRatioToColor(speedRatio) : 0x555555;
					const bwColor = reverseSpeedRatio !== undefined ? speedRatioToColor(reverseSpeedRatio) : 0x555555;
					g.setFillStyle({ color: fwColor });
					g.rect(from, 0, visibleLength, fwWidth);
					g.fill();
					g.setFillStyle({ color: bwColor });
					g.rect(from, -bwWidth, visibleLength, bwWidth);
					g.fill();
				} else {
					const surfaceColor = densityView && speedRatio !== undefined
						? speedRatioToColor(speedRatio)
						: 0x555555;
					g.setFillStyle({ color: surfaceColor });
					g.rect(from, -bwWidth, visibleLength, fwWidth + bwWidth);
					g.fill();
				}

				// Lane dividers (dashed white)
				for (let i = 1; i < fwCount; i++) {
					drawDashedLine(g, from, i * laneWidth, to, i * laneWidth, 0xffffff, 1);
				}
				for (let i = 1; i < bwCount; i++) {
					drawDashedLine(g, from, -i * bwLaneWidth, to, -i * bwLaneWidth, 0xffffff, 1);
				}

				// Center line
				if (isTwoWay) {
					// Double yellow — opposing traffic
					g.setStrokeStyle({ color: 0xf59e0b, width: 1 });
					g.moveTo(from, -1); g.lineTo(to, -1); g.stroke();
					g.moveTo(from,  1); g.lineTo(to,  1); g.stroke();
				}

				// Outer borders are drawn once around the union of roads and junctions.

				// Direction arrow for one-way roads (always points toward endNode)
				if (!isTwoWay && visibleLength >= 16) {
					const arrowX = (from + to) / 2;
					const arrowY = fwWidth / 2;
					const arrowSize = 6;
					g.setFillStyle({ color: 0x888888 });
					g.moveTo(arrowX + arrowSize, arrowY);
					g.lineTo(arrowX - arrowSize, arrowY - arrowSize / 2);
					g.lineTo(arrowX - arrowSize, arrowY + arrowSize / 2);
					g.fill();
				}
			}}
		/>
	);
});
