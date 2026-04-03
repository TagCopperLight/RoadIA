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
    heading?: number;
    speed?: number;
}

export interface ScoreData {
	score: number;
	total_trip_time: number;
	total_emitted_co2: number;
	network_length: number;
	total_distance_traveled: number;
	success_rate: number;
}
