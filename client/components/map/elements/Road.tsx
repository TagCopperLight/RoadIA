import { MapNode } from '../types';

export function Road({ start, end, laneCount }: { start: MapNode, end: MapNode, laneCount: number }) {
	const laneWidth = 7;
	const lineWidth = 1;
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

			for (let line = -laneCount+1; line < laneCount; line++){
				const linePosition = line*(laneWidth+lineWidth);
				graphics.setStrokeStyle({ color: 'white' });
				graphics.moveTo(0, linePosition);
				graphics.lineTo(length, linePosition);
				graphics.stroke({width: lineWidth});
			}
		}} />
	);
}