import { describe, expect, it } from 'vitest';
import { buildRoadNetworkGeometry } from './roadNetworkGeometry';
import { MapEdge, MapNode } from './types';

const node = (id: number, x: number, y: number): MapNode => ({ id, x, y, radius: 10, kind: 'Intersection', name: '' });
const edge = (id: number, from: number, to: number): MapEdge => ({ id, from, to, lane_count: 1, lane_width: 3.5, length: 100, speed_limit: 50 });

describe('shared pavement outline', () => {
	it('removes borders across road mouths and keeps dead-end caps', () => {
		const nodes = new Map([node(1, 0, 0), node(2, 100, 0)].map(n => [n.id, n]));
		const geometry = buildRoadNetworkGeometry(nodes, [{ canonical: edge(1, 1, 2), reverse: edge(2, 2, 1) }]);
		expect(geometry.outline).toHaveLength(1);
		const ring = geometry.outline[0][0];
		expect(Math.min(...ring.map(p => p[0]))).toBeLessThan(0);
		expect(Math.max(...ring.map(p => p[0]))).toBeGreaterThan(100);
		for (let i = 1; i < ring.length; i++) {
			const a = ring[i - 1];
			const b = ring[i];
			if (a[0] === b[0] && (a[0] === 10 || a[0] === 90)) {
				expect(Math.abs(a[1] - b[1])).toBeLessThan(1);
			}
		}
	});

	it('removes internal borders where unrelated road strips overlap', () => {
		const nodes = new Map([node(1, -50, 0), node(2, 50, 0), node(3, 0, -50), node(4, 0, 50)].map(n => [n.id, n]));
		const geometry = buildRoadNetworkGeometry(nodes, [
			{ canonical: edge(1, 1, 2), reverse: edge(2, 2, 1) },
			{ canonical: edge(3, 3, 4), reverse: edge(4, 4, 3) },
		]);
		expect(geometry.outline).toHaveLength(1);
		const ring = geometry.outline[0][0];
		for (let i = 1; i < ring.length; i++) {
			const midX = (ring[i - 1][0] + ring[i][0]) / 2;
			const midY = (ring[i - 1][1] + ring[i][1]) / 2;
			expect(Math.abs(midX) < 3.49 && Math.abs(midY) < 3.49).toBe(false);
		}
	});

	it.each([0, 0.0002, 0.002, 0.02])('keeps pavement around the node and along approaches separated by %s radians', angle => {
		const nodes = new Map([node(1, 0, 0), node(2, 100, 0), node(3, Math.cos(angle) * 100, Math.sin(angle) * 100)].map(n => [n.id, n]));
		const geometry = buildRoadNetworkGeometry(nodes, [
			{ canonical: edge(1, 1, 2), reverse: edge(2, 2, 1) },
			{ canonical: edge(3, 1, 3), reverse: edge(4, 3, 1) },
		]);
		function contains(ring: number[][], x: number, y: number) {
			let inside = false;
			for (let i = 0, j = ring.length - 1; i < ring.length; j = i++) {
				const a = ring[i], b = ring[j];
				if ((a[1] > y) !== (b[1] > y) && x < (b[0] - a[0]) * (y - a[1]) / (b[1] - a[1]) + a[0]) inside = !inside;
			}
			return inside;
		}
		for (const [x, y] of [[0, 0], [25, 0], [25 * Math.cos(angle), 25 * Math.sin(angle)]]) {
			expect(geometry.outline.some(polygon => contains(polygon[0], x, y) && !polygon.slice(1).some(hole => contains(hole, x, y)))).toBe(true);
		}
	});

	it('keeps the grass island inside a ring of roads', () => {
		const nodes = new Map([node(1, 0, 0), node(2, 100, 0), node(3, 100, 100), node(4, 0, 100)].map(n => [n.id, n]));
		const pairs = [[1, 2], [2, 3], [3, 4], [4, 1]].map(([from, to], i) => ({ canonical: edge(i * 2, from, to), reverse: edge(i * 2 + 1, to, from) }));
		const geometry = buildRoadNetworkGeometry(nodes, pairs);
		expect(geometry.outline).toHaveLength(1);
		expect(geometry.outline[0]).toHaveLength(2);
	});
});
