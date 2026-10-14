import { expect, it } from 'vitest';
import { Graphics } from 'pixi.js';
import { ReactElement } from 'react';
import { TrafficLightIndicator } from './TrafficLightIndicator';

it('positions the signal before an expanded junction mouth', () => {
	const component = TrafficLightIndicator as unknown as { type: (props: object) => ReactElement<{ draw: (g: Graphics) => void }> };
	const element = component.type({
		start: { x: 0, y: 0, radius: 10 }, end: { x: 100, y: 0, radius: 10 },
		edge: { lane_count: 1, lane_width: 3.5 }, isGreen: true, endReach: 30,
	});
	const graphics = new Graphics();
	element.props.draw(graphics);
	// Stop line at x=66; the signal circle extends four units on either side.
	expect(graphics.context.bounds.maxX).toBeLessThanOrEqual(70);
	graphics.destroy();
});
