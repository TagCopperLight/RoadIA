import { describe, expect, it } from 'vitest';
import { Graphics } from 'pixi.js';
import { ReactElement } from 'react';
import { Road } from './Road';
import { MapNode, MapEdge } from '../types';

const start: MapNode = { id: 1, x: 0, y: 0, radius: 10, kind: 'Intersection', name: '' };
const end: MapNode = { ...start, id: 2, x: 100 };
const edge: MapEdge = { id: 1, from: 1, to: 2, lane_count: 2, lane_width: 3.5, length: 100, speed_limit: 50 };
const reverse: MapEdge = { ...edge, id: 2, from: 2, to: 1 };

function drawRoad(endNode = end) {
	const component = Road as unknown as { type: (props: object) => ReactElement<{ draw: (g: Graphics) => void }> };
	const element = component.type({ canonicalEdge: edge, reverseEdge: reverse, startNode: start, endNode });
	const graphics = new Graphics();
	element.props.draw(graphics);
	return graphics;
}

describe('road rendering at junction boundaries', () => {
	it('stops the road surface and markings at both junction mouths', () => {
		const graphics = drawRoad();
		const bounds = graphics.context.bounds;
		expect(bounds.minX).toBeGreaterThanOrEqual(9);
		expect(bounds.maxX).toBeLessThanOrEqual(91);
		graphics.destroy();
	});

	it('omits the road strip when the two junction mouths consume a short road', () => {
		const graphics = drawRoad({ ...end, x: 6 });
		expect(graphics.context.instructions).toHaveLength(0);
		graphics.destroy();
	});
});
