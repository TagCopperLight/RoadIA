export interface InternalLane {
	id: number;
	entry: [number, number];
	exit: [number, number];
	link_type: "Priority" | "Yield" | "Stop" | "TrafficLight";
}

export interface MapNode {
	id: number;
	kind: "Intersection" | "Habitation" | "Workplace";
	name: string;
	x: number;
	y: number;
	has_traffic_light?: boolean;
	radius: number;
	internal_lanes?: InternalLane[];
}

export interface MapEdge {
	from: number;
	id: number;
	lane_count: number;
	lane_width: number;
	length: number;
	speed_limit: number;
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
    vehicle_type?: string;         // ElectricSmall | StandardHybrid | StandardDiesel | StandardThermal
    motorization?: string;          // EssenceHybride | Electrique | EssenceThermal | Diesel
    heading?: number;
}

export interface TrafficLightData {
    id: number;            // intersection_id
    green_road_ids: number[];  // road IDs with a green approach into this intersection
}

export interface ScoreData {
	score: number;
	total_trip_time: number;
	ref_total_trip_time: number;
	total_emitted_co2: number;
	ref_total_emitted_co2: number;
	network_length: number;
	ref_network_length: number;
	success_rate: number;
}
