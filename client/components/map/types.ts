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

export type Motorization = "Hybride" | "Electrique" | "Essence" | "Diesel";

export interface VehicleData {
    id: number;
    x: number;
    y: number;
    kind: string;
    state: string;
    heading?: number;
    motorization?: Motorization;
    origin_id?: number;
    destination_id?: number;
    waypoint_ids?: number[];
	commute_plan_id: number | null;
}

export interface VehicleUpdatePacket {
	vehicles: VehicleData[];
	traffic_lights: TrafficLightData[];
	simulation_time_s: number;
}

export interface VehicleSummary {
    id: number;
    origin_id: number;
    destination_id: number;
    motorization: Motorization;
    waypoint_ids: number[];
	commute_plan_id: number | null;
}

export interface TrafficLightData {
    id: number;            // intersection_id
    green_road_ids: number[];  // road IDs with a green approach into this intersection
}

export interface RoadMetricData {
	id: number;
	speed_ratio: number;
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
