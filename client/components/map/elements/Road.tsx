import { memo } from 'react';
import { Graphics } from 'pixi.js';
import { MapNode, MapEdge } from '../types';

interface RoadProps {
	canonicalEdge: MapEdge;  // startNode → endNode; lanes on positive y (right of travel)
	reverseEdge?: MapEdge;   // endNode → startNode; lanes on negative y (two-way only)
	startNode: MapNode;
	endNode: MapNode;
	isSelected?: boolean;
	isEditMode?: boolean;
	onSelect?: () => void;
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

export const Road = memo(function Road({ canonicalEdge, reverseEdge, startNode, endNode, isSelected, isEditMode, onSelect }: RoadProps) {
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
				const bwWidth = bwCount * laneWidth;

				// Selection highlight behind road
				if (isSelected) {
					g.setFillStyle({ color: 0xfbbf24, alpha: 0.4 });
					g.rect(-4, -bwWidth - 4, length + 8, fwWidth + bwWidth + 8);
					g.fill();
				}

				// Road surface (asphalt gray)
				g.setFillStyle({ color: 0x555555 });
				g.rect(0, -bwWidth, length, fwWidth + bwWidth);
				g.fill();

				// Lane dividers (dashed white)
				for (let i = 1; i < fwCount; i++) {
					drawDashedLine(g, 0, i * laneWidth, length, i * laneWidth, 0xffffff, 1);
				}
				for (let i = 1; i < bwCount; i++) {
					drawDashedLine(g, 0, -i * laneWidth, length, -i * laneWidth, 0xffffff, 1);
				}

				// Center line
				if (isTwoWay) {
					// Double yellow — opposing traffic
					g.setStrokeStyle({ color: 0xf59e0b, width: 1 });
					g.moveTo(0, -1); g.lineTo(length, -1); g.stroke();
					g.moveTo(0,  1); g.lineTo(length,  1); g.stroke();
				} else {
					// Single white left edge for one-way roads
					g.setStrokeStyle({ color: 0xffffff, width: 1 });
					g.moveTo(0, 0); g.lineTo(length, 0); g.stroke();
				}

				// Outer road edges (solid white)
				g.setStrokeStyle({ color: 0xffffff, width: 2 });
				g.moveTo(0, fwWidth); g.lineTo(length, fwWidth); g.stroke();
				if (bwWidth > 0) {
					g.moveTo(0, -bwWidth); g.lineTo(length, -bwWidth); g.stroke();
				}

				// Direction arrow for one-way roads (always points toward endNode)
				if (!isTwoWay) {
					const arrowX = length / 2;
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
