import polygonClipping from 'polygon-clipping';
import type { MultiPolygon, Pair, Polygon, Ring } from 'polygon-clipping';
import { MapEdge, MapNode } from './types';
import { addRoadApproaches, buildJunctionCorners, JunctionApproach, JunctionCorner } from './intersectionGeometry';

export interface RoadPair { canonical: MapEdge; reverse?: MapEdge }
export interface RoadNetworkGeometry {
	junctions: Map<number, JunctionCorner[]>;
	outline: MultiPolygon;
}

// Shared mouth vertices must compare exactly in the polygon union, even when
// the two ends of a road computed their coordinates in different directions.
function point(x: number, y: number): Pair {
	return [Math.round(x * 1e6) / 1e6, Math.round(y * 1e6) / 1e6];
}

export function buildRoadNetworkGeometry(nodes: Map<number, MapNode>, pairs: RoadPair[]): RoadNetworkGeometry {
	const approaches = new Map<number, JunctionApproach[]>();
	for (const { canonical, reverse } of pairs) {
		const start = nodes.get(canonical.from);
		const end = nodes.get(canonical.to);
		if (start && end) addRoadApproaches(approaches, start, end, canonical, reverse);
	}
	const junctions = new Map<number, JunctionCorner[]>();
	const polygons: Polygon[] = [];
	for (const node of nodes.values()) {
		const corners = buildJunctionCorners(node.radius, approaches.get(node.id) ?? []);
		junctions.set(node.id, corners);
		const ring: Ring = [];
		if (corners.length) {
			for (const { left, right, control, nextLeft } of corners) {
				ring.push(point(node.x + left.x, node.y + left.y), point(node.x + right.x, node.y + right.y));
				for (let step = 1; step <= 16; step++) {
					const t = step / 16;
					const u = 1 - t;
					ring.push(point(
						node.x + u * u * right.x + 2 * u * t * control.x + t * t * nextLeft.x,
						node.y + u * u * right.y + 2 * u * t * control.y + t * t * nextLeft.y,
					));
				}
			}
		} else if (node.radius > 0) {
			for (let i = 0; i < 32; i++) {
				const angle = i * Math.PI / 16;
				ring.push(point(node.x + Math.cos(angle) * node.radius, node.y + Math.sin(angle) * node.radius));
			}
		}
		if (ring.length >= 3) polygons.push([ring]);
	}
	for (const { canonical } of pairs) {
		const start = nodes.get(canonical.from);
		const end = nodes.get(canonical.to);
		const a = junctions.get(canonical.from)?.find(c => c.roadId === canonical.id);
		const b = junctions.get(canonical.to)?.find(c => c.roadId === canonical.id);
		if (!start || !end || !a || !b) continue;
		if (Math.hypot(end.x - start.x, end.y - start.y) - a.reach - b.reach <= 0.001) continue;
		polygons.push([[
			point(start.x + a.left.x, start.y + a.left.y),
			point(start.x + a.right.x, start.y + a.right.y),
			point(end.x + b.left.x, end.y + b.left.y),
			point(end.x + b.right.x, end.y + b.right.y),
		]]);
	}
	// Stroke only this boundary: individual curbs would remain visible inside
	// overlapping junctions, short roads, or closely spaced parallel branches.
	const outline = polygons.length ? polygonClipping.union(polygons[0], ...polygons.slice(1)) : [];
	return { junctions, outline };
}
