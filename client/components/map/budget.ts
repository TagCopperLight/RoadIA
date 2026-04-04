import { MapData, MapNode, MapEdge } from './types';

export const MAX_BUDGET = 750_000_000;

const BASE_COST_PER_METER = 500;

const BASE_INTERSECTION_COST: Record<MapNode['kind'], number> = {
    Intersection: 50_000,
    Habitation:   150_000,
    Workplace:    200_000,
};

const RADIUS_COST_PER_METER = 2_000;

export function roadCost(edge: MapEdge): number {
    return BASE_COST_PER_METER * edge.length * edge.lane_count;
}

export function nodeCost(node: MapNode): number {
    return BASE_INTERSECTION_COST[node.kind] + RADIUS_COST_PER_METER * node.radius;
}

export function calculateCost(mapData: MapData): number {
    return mapData.edges.reduce((sum, e) => sum + roadCost(e), 0)
         + mapData.nodes.reduce((sum, n) => sum + nodeCost(n), 0);
}

// Estimates road cost before server creates it, using node coordinates
export function estimateRoadCost(fromNode: MapNode, toNode: MapNode, laneCount = 2): number {
    const dx = toNode.x - fromNode.x;
    const dy = toNode.y - fromNode.y;
    const estimatedLength = Math.max(0, Math.sqrt(dx * dx + dy * dy) - fromNode.radius - toNode.radius);
    return BASE_COST_PER_METER * estimatedLength * laneCount;
}

// Estimates node cost before server creates it (uses minimum radius for fresh nodes)
export function estimateNodeCost(kind: MapNode['kind'] = 'Intersection'): number {
    return BASE_INTERSECTION_COST[kind] + RADIUS_COST_PER_METER * 8;
}
