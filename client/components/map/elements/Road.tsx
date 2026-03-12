import { MapNode } from '../types';
import {Graphics} from "pixi.js";

const laneWidth = 7;
const lineWidth = 1;
const lineLength = 8;

export function Road({ start, end, laneCount }: { start: MapNode, end: MapNode, laneCount: number }) {
	const width = (laneWidth + lineWidth) * 2 * laneCount - lineWidth;
	return (
		<pixiGraphics draw={(graphics) => {
			graphics.clear();

			const dx = end.x - start.x;
			const dy = end.y - start.y;
			const length = Math.sqrt(dx * dx + dy * dy);
			const angle = Math.atan2(dy, dx);

			graphics.position.set(start.x, start.y);
			graphics.rotation = angle;

			graphics.setFillStyle({ color: 'gray' });
			graphics.rect(0, -width / 2, length, width);
			graphics.fill();

			graphics.setStrokeStyle({ color: 'white' });
			graphics.moveTo(0, 0);
			graphics.lineTo(length, 0);
			graphics.stroke({width: lineWidth});

			for (let line = -1; line > -laneCount; line--){
				drawDashedLine(graphics, line, length);
			}
			for (let line = 1; line < laneCount; line++){
				drawDashedLine(graphics, line, length);
			}
		}} />
	);
}

function drawDashedLine(graphics: Graphics, line: number, length: number){
	const linePosition = line*(laneWidth+lineWidth);
	graphics.setStrokeStyle({ color: 'white' });

	for(let dash = lineLength; dash + lineLength < length; dash += lineLength*2){
		graphics.moveTo(dash, linePosition);
		graphics.lineTo(dash + lineLength, linePosition);
	}

	graphics.stroke({width: lineWidth});
}