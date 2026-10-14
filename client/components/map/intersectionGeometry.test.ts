import { describe, expect, it } from 'vitest';
import { addRoadApproaches, buildJunctionCorners, JunctionApproach } from './intersectionGeometry';
import { MapEdge, MapNode } from './types';

const approach = (x: number, y: number): JunctionApproach => ({ x, y, length: 100, leftWidth: 3.5, rightWidth: 3.5 });

describe('procedural junction geometry', () => {
	it('matches all four road mouths and rounds corners inward at a crossroads', () => {
		const corners = buildJunctionCorners(10, [approach(1, 0), approach(0, 1), approach(-1, 0), approach(0, -1)]);
		expect(corners).toHaveLength(4);
		for (const corner of corners) {
			expect(Math.abs(corner.control.x)).toBe(3.5);
			expect(Math.abs(corner.control.y)).toBe(3.5);
			expect(Math.hypot(corner.right.x - corner.left.x, corner.right.y - corner.left.y)).toBe(7);
		}
	});

	it('moves acute-angle mouths beyond the point where road edges overlap', () => {
		const corners = buildJunctionCorners(10, [approach(1, 0), approach(Math.cos(Math.PI / 6), Math.sin(Math.PI / 6))]);
		const east = corners.find(corner => corner.left.y === -3.5)!;
		// Facing edges of 7-unit-wide roads at 30 degrees meet at x=13.062.
		expect(east.right.x).toBeGreaterThan(13.062);
	});

	it('keeps a straight two-road connection straight', () => {
		const corners = buildJunctionCorners(10, [approach(1, 0), approach(-1, 0)]);
		for (const corner of corners) {
			expect(corner.control.y).toBe(corner.right.y);
			expect(corner.nextLeft.y).toBe(corner.right.y);
		}
	});

	it('uses each direction’s lane widths and flips sides at the far endpoint', () => {
		const start: MapNode = { id: 1, x: 0, y: 0, radius: 10, kind: 'Intersection', name: '' };
		const end = { ...start, id: 2, x: 100 };
		const road: MapEdge = { id: 1, from: 1, to: 2, lane_count: 2, lane_width: 3, length: 100, speed_limit: 50 };
		const reverse = { ...road, id: 2, from: 2, to: 1, lane_count: 1, lane_width: 4 };
		const approaches = new Map<number, JunctionApproach[]>();
		addRoadApproaches(approaches, start, end, road, reverse);
		expect(approaches.get(1)?.[0]).toMatchObject({ leftWidth: 4, rightWidth: 6 });
		expect(approaches.get(2)?.[0]).toMatchObject({ leftWidth: 6, rightWidth: 4 });
		const oneWay = new Map<number, JunctionApproach[]>();
		addRoadApproaches(oneWay, start, end, road);
		expect(oneWay.get(1)?.[0].leftWidth).toBe(0);
		expect(oneWay.get(2)?.[0].rightWidth).toBe(0);
		addRoadApproaches(oneWay, start, { ...end, x: 0 }, road);
		expect(oneWay.get(1)).toHaveLength(1);
	});

	it('handles isolated nodes, dead ends, short roads, and nearly parallel approaches without spikes', () => {
		expect(buildJunctionCorners(10, [])).toEqual([]);
		const cap = buildJunctionCorners(10, [approach(1, 0)])[0];
		expect(cap.control.x).toBeLessThan(0);
		const short = buildJunctionCorners(10, [{ ...approach(1, 0), length: 6 }])[0];
		expect(short.left.x).toBe(3);
		const corners = buildJunctionCorners(10, [approach(1, 0), approach(Math.cos(0.002), Math.sin(0.002))]);
		for (const corner of corners) {
			expect(Number.isFinite(corner.control.x)).toBe(true);
			expect(Math.hypot(corner.control.x, corner.control.y)).toBeLessThanOrEqual(60);
		}
	});
});
