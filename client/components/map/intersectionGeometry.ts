import { MapEdge, MapNode } from './types';

export interface JunctionApproach {
	roadId?: number;
	x: number;
	y: number;
	length: number;
	leftWidth: number;
	rightWidth: number;
}

export interface JunctionPoint { x: number; y: number }
export interface JunctionCorner {
	roadId?: number;
	reach: number;
	left: JunctionPoint;
	right: JunctionPoint;
	control: JunctionPoint;
	nextLeft: JunctionPoint;
}

// Use the same paired road orientation as Road, including asymmetric and one-way roads.
export function addRoadApproaches(
	approaches: Map<number, JunctionApproach[]>,
	start: MapNode,
	end: MapNode,
	canonical: MapEdge,
	reverse?: MapEdge,
) {
	const dx = end.x - start.x;
	const dy = end.y - start.y;
	const length = Math.hypot(dx, dy);
	if (length < 0.001) return;
	const x = dx / length;
	const y = dy / length;
	const rightWidth = canonical.lane_count * canonical.lane_width;
	const leftWidth = reverse ? reverse.lane_count * reverse.lane_width : 0;
	for (const [id, approach] of [
		[start.id, { roadId: canonical.id, x, y, length, leftWidth, rightWidth }],
		[end.id, { roadId: canonical.id, x: -x, y: -y, length, leftWidth: rightWidth, rightWidth: leftWidth }],
	] as const) {
		const list = approaches.get(id) ?? [];
		list.push(approach);
		approaches.set(id, list);
	}
}

export function junctionReach(radius: number, length: number, leftWidth: number, rightWidth: number): number {
	return Math.min(Math.max(radius, leftWidth, rightWidth, 2), length / 2);
}

export function buildJunctionCorners(radius: number, approaches: JunctionApproach[]): JunctionCorner[] {
	const sorted = [...approaches].sort((a, b) => Math.atan2(a.y, a.x) - Math.atan2(b.y, b.x));
	const reaches = sorted.map(a => junctionReach(radius, a.length, a.leftWidth, a.rightWidth));
	// Facing borders at a sharp fork meet further out than the node radius.
	// Put the mouths beyond that meeting point, rather than crossing their curbs.
	for (let i = 0; i < sorted.length && sorted.length > 1; i++) {
		const a = sorted[i];
		const j = (i + 1) % sorted.length;
		const b = sorted[j];
		const cross = a.x * b.y - a.y * b.x;
		if (cross <= 0.001) continue;
		const dx = b.y * b.leftWidth + a.y * a.rightWidth;
		const dy = -b.x * b.leftWidth - a.x * a.rightWidth;
		const t = (dx * b.y - dy * b.x) / cross;
		const u = (dx * a.y - dy * a.x) / cross;
		if (t >= 0 && u >= 0) {
			reaches[i] = Math.min(a.length / 2, Math.max(reaches[i], t + 2));
			reaches[j] = Math.min(b.length / 2, Math.max(reaches[j], u + 2));
		}
	}
	const mouths = sorted.map((approach, i) => {
		const { x, y, leftWidth, rightWidth } = approach;
		const reach = reaches[i];
		return {
			...approach, reach,
			left: { x: x * reach + y * leftWidth, y: y * reach - x * leftWidth },
			right: { x: x * reach - y * rightWidth, y: y * reach + x * rightWidth },
		};
	});

	return mouths.map((mouth, index) => {
		const next = mouths[(index + 1) % mouths.length];
		const a = mouth.right;
		const b = next.left;
		let control = { x: (a.x + b.x) / 2, y: (a.y + b.y) / 2 };
		if (mouths.length === 1) {
			// Rounded cap at a dead end, centered on the actual lane bundle.
			const offset = (mouth.rightWidth - mouth.leftWidth) / 2;
			const depth = Math.max((mouth.rightWidth + mouth.leftWidth) / 2, 2);
			control = { x: -mouth.x * (mouth.reach + depth * 2) - mouth.y * offset, y: -mouth.y * (mouth.reach + depth * 2) + mouth.x * offset };
		} else {
			const angle = Math.atan2(mouth.y, mouth.x);
			const gap = Math.atan2(next.y, next.x) - angle + (index === mouths.length - 1 ? Math.PI * 2 : 0);
			if (gap > Math.PI + 0.001) {
				// A reflex corner must wrap behind the node. Joining distant mouths
				// by their midpoint can collapse an almost parallel fork to a sliver.
				const depth = Math.max(mouth.rightWidth, next.leftWidth, 2);
				const bisector = angle + gap / 2;
				control = {
					x: -control.x + Math.cos(bisector) * depth * 2,
					y: -control.y + Math.sin(bisector) * depth * 2,
				};
			}
			const cross = mouth.x * next.y - mouth.y * next.x;
			if (Math.abs(cross) > 0.001) {
				const dx = b.x - a.x;
				const dy = b.y - a.y;
				const t = (dx * next.y - dy * next.x) / cross;
				const s = (dx * mouth.y - dy * mouth.x) / cross;
				const candidate = { x: a.x + t * mouth.x, y: a.y + t * mouth.y };
				// Near-parallel or overlapping approaches must not create long spikes.
				const limit = Math.max(mouth.reach, next.reach, mouth.leftWidth, mouth.rightWidth, next.leftWidth, next.rightWidth) * 2;
				if (t <= 0 && s <= 0 && Math.hypot(candidate.x, candidate.y) <= limit) control = candidate;
			}
		}
		return { roadId: mouth.roadId, reach: mouth.reach, left: mouth.left, right: a, control, nextLeft: b };
	});
}
