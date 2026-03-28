import { MapNode } from '../types';

export function Intersection({ node }: { node: MapNode }) {
	return (
		<pixiGraphics draw={(graphics) => {
			graphics.clear();
			graphics.position.set(node.x, node.y);
			graphics.setFillStyle({ color: node.kind === 'Habitation' ? 'blue' : node.kind === 'Workplace' ? 'red' : 'lightgray' });
			graphics.circle(0, 0, 2);
			graphics.fill();
		}} />
	);
}
