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
}
