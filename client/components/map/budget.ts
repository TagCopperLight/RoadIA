import { MapData, MapNode, MapEdge } from './types';

export interface BudgetConfig {
    max_budget: number;
    base_cost_per_meter: number;
    intersection_cost: number;
    habitation_cost: number;
    workplace_cost: number;
}

export const DEFAULT_BUDGET_CONFIG: BudgetConfig = {
    max_budget: 750_000_000,
    base_cost_per_meter: 500,
    intersection_cost: 50_000,
    habitation_cost: 150_000,
    workplace_cost: 200_000,
};

const RADIUS_COST_PER_METER = 2_000;

export function roadCost(edge: MapEdge, cfg: BudgetConfig = DEFAULT_BUDGET_CONFIG): number {
    return cfg.base_cost_per_meter * edge.length * edge.lane_count;
}

export function nodeCost(node: MapNode, cfg: BudgetConfig = DEFAULT_BUDGET_CONFIG): number {
    const baseCost: Record<MapNode['kind'], number> = {
        Intersection: cfg.intersection_cost,
        Habitation: cfg.habitation_cost,
        Workplace: cfg.workplace_cost,
    };
    return baseCost[node.kind] + RADIUS_COST_PER_METER * node.radius;
}

export function calculateCost(mapData: MapData, cfg: BudgetConfig = DEFAULT_BUDGET_CONFIG): number {
    return mapData.edges.reduce((sum, e) => sum + roadCost(e, cfg), 0)
         + mapData.nodes.reduce((sum, n) => sum + nodeCost(n, cfg), 0);
}

export function estimateRoadCost(fromNode: MapNode, toNode: MapNode, laneCount = 2, cfg: BudgetConfig = DEFAULT_BUDGET_CONFIG): number {
    const dx = toNode.x - fromNode.x;
    const dy = toNode.y - fromNode.y;
    const estimatedLength = Math.max(0, Math.sqrt(dx * dx + dy * dy) - fromNode.radius - toNode.radius);
    return cfg.base_cost_per_meter * estimatedLength * laneCount;
}

export function estimateNodeCost(kind: MapNode['kind'] = 'Intersection', cfg: BudgetConfig = DEFAULT_BUDGET_CONFIG): number {
    const baseCost: Record<MapNode['kind'], number> = {
        Intersection: cfg.intersection_cost,
        Habitation: cfg.habitation_cost,
        Workplace: cfg.workplace_cost,
    };
    return baseCost[kind] + RADIUS_COST_PER_METER * 8;
}
