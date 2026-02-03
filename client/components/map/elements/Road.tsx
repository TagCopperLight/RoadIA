import { MapNode } from '../types';

export function Road({ start, end }: { start: MapNode, end: MapNode }) {
	const width = 15;
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
			graphics.stroke();
		}} />
	);
}
