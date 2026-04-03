export interface MapNode {
	id: number;
	kind: "Intersection" | "Habitation" | "Workplace";
	name: string;
	x: number;
	y: number;
}

export interface MapEdge {
	from: number;
	id: number;
	lane_count: number;
	length: number;
	to: number;
	speed_limit: number;
	is_blocked: boolean;
	can_overtake: boolean;
	intersection_type?: 'Priority' | 'Yield' | 'Stop';
}

export interface MapData {
	nodes: MapNode[];
	edges: MapEdge[];
}

export interface VehicleData {
    id: number;
    x: number;
    y: number;
    kind: string;
    state: string;
    heading?: number;
    speed?: number;
}

export type EditTool = "select" | "addNode" | "addRoad" | "navigate";
