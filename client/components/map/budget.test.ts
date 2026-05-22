import { describe, it, expect } from 'vitest';
import {
    roadCost,
    nodeCost,
    calculateCost,
    estimateRoadCost,
    estimateNodeCost,
    DEFAULT_BUDGET_CONFIG,
} from './budget';
import { MapData, MapNode, MapEdge } from './types';

describe('budget module unit tests', () => {
    describe('roadCost', () => {
        it('should calculate road cost based on length, lane count, and cost per meter', () => {
            const edge: MapEdge = {
                id: 1,
                from: 0,
                to: 1,
                lane_count: 2,
                lane_width: 3.5,
                length: 100,
                speed_limit: 50,
            };
            const cost = roadCost(edge);
            // 500 * 100 * 2 = 100,000
            expect(cost).toBe(100000);
        });

        it('should respect custom budget config for road cost', () => {
            const edge: MapEdge = {
                id: 1,
                from: 0,
                to: 1,
                lane_count: 3,
                lane_width: 3.5,
                length: 50,
                speed_limit: 50,
            };
            const customCfg = {
                ...DEFAULT_BUDGET_CONFIG,
                base_cost_per_meter: 1000,
            };
            const cost = roadCost(edge, customCfg);
            // 1000 * 50 * 3 = 150,000
            expect(cost).toBe(150000);
        });
    });

    describe('nodeCost', () => {
        it('should calculate intersection cost including radius penalty', () => {
            const node: MapNode = {
                id: 1,
                x: 10,
                y: 20,
                kind: 'Intersection',
                name: 'Intersection Node',
                radius: 5,
            };
            const cost = nodeCost(node);
            // 50,000 (intersection_cost) + 2000 * 5 (radius) = 60,000
            expect(cost).toBe(60000);
        });

        it('should calculate habitation cost including radius penalty', () => {
            const node: MapNode = {
                id: 2,
                x: 10,
                y: 20,
                kind: 'Habitation',
                name: 'Habitation Node',
                radius: 10,
            };
            const cost = nodeCost(node);
            // 150,000 (habitation_cost) + 2000 * 10 = 170,000
            expect(cost).toBe(170000);
        });

        it('should calculate workplace cost including radius penalty', () => {
            const node: MapNode = {
                id: 3,
                x: 10,
                y: 20,
                kind: 'Workplace',
                name: 'Workplace Node',
                radius: 0,
            };
            const cost = nodeCost(node);
            // 200,000 (workplace_cost) + 2000 * 0 = 200,000
            expect(cost).toBe(200000);
        });
    });

    describe('calculateCost', () => {
        it('should sum all nodes and edges correctly', () => {
            const mapData: MapData = {
                nodes: [
                    { id: 1, x: 0, y: 0, kind: 'Habitation', name: 'Hab', radius: 5 }, // 150k + 10k = 160k
                    { id: 2, x: 100, y: 0, kind: 'Workplace', name: 'Work', radius: 10 }, // 200k + 20k = 220k
                ],
                edges: [
                    { id: 1, from: 1, to: 2, lane_count: 2, lane_width: 3.5, length: 100, speed_limit: 50 }, // 500 * 100 * 2 = 100k
                ],
            };

            const total = calculateCost(mapData);
            // 160k + 220k + 100k = 480k
            expect(total).toBe(480000);
        });
    });

    describe('estimateRoadCost', () => {
        it('should estimate road cost based on Euclidean distance minus node radii', () => {
            const fromNode: MapNode = { id: 1, x: 0, y: 0, kind: 'Intersection', name: 'From', radius: 10 };
            const toNode: MapNode = { id: 2, x: 120, y: 0, kind: 'Intersection', name: 'To', radius: 10 };
            // Distance = 120. Net distance = 120 - 10 - 10 = 100.
            const estCost = estimateRoadCost(fromNode, toNode, 2);
            // 500 * 100 * 2 = 100,000
            expect(estCost).toBe(100000);
        });

        it('should return 0 cost if nodes overlap or distance is less than sum of radii', () => {
            const fromNode: MapNode = { id: 1, x: 0, y: 0, kind: 'Intersection', name: 'From', radius: 10 };
            const toNode: MapNode = { id: 2, x: 15, y: 0, kind: 'Intersection', name: 'To', radius: 10 };
            // Distance = 15. Net distance = 15 - 10 - 10 = -5 -> clamped to 0.
            const estCost = estimateRoadCost(fromNode, toNode, 2);
            expect(estCost).toBe(0);
        });
    });

    describe('estimateNodeCost', () => {
        it('should estimate cost for nodes assuming a default radius of 8', () => {
            const estHab = estimateNodeCost('Habitation');
            // 150,000 (habitation_cost) + 2_000 * 8 = 166,000
            expect(estHab).toBe(166000);
        });
    });
});
