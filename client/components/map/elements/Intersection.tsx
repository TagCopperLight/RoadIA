import { memo } from 'react';
import { MapNode } from '../types';

export const Intersection = memo(function Intersection({ node }: { node: MapNode }) {
	return (
		<pixiGraphics draw={(g) => {
			g.clear();
			g.position.set(node.x, node.y);

			g.setFillStyle({ color: 0x555555 });
			g.circle(0, 0, node.radius);
			g.fill();

			const ringColor = node.kind === 'Habitation' ? 0x3b82f6
				: node.kind === 'Workplace' ? 0xef4444
				: 0x888888;
			g.setStrokeStyle({ color: ringColor, width: 2 });
			g.circle(0, 0, node.radius);
			g.stroke();
		}} />
	);
});
