import { useMemo } from 'react';
import { useApplication } from '@pixi/react';
import { MapData, MapEdge, VehicleData, TrafficLightData } from './types';
import { Road } from './elements/Road';
import { Intersection } from './elements/Intersection';
import { Vehicle } from './elements/Vehicle';
import { TrafficLightIndicator } from './elements/TrafficLightIndicator';

export function MapCanvas({
	data,
	vehicles,
	trafficLights,
}: {
	data: MapData;
	vehicles: VehicleData[];
	trafficLights: Map<number, TrafficLightData>;
}) {
	const { app } = useApplication();

	const nodeMap = useMemo(
		() => new Map(data.nodes.map(n => [n.id, n])),
		[data.nodes]
	);

	const edgePairs = useMemo(() => {
		const map = new Map<string, { canonical: MapEdge; reverse?: MapEdge }>();
		for (const edge of data.edges) {
			const key = `${Math.min(edge.from, edge.to)}-${Math.max(edge.from, edge.to)}`;
			const entry = map.get(key);
			if (!entry) {
				map.set(key, { canonical: edge });
			} else if (edge.from === entry.canonical.to) {
				entry.reverse = edge;
			}
		}
		return map;
	}, [data.edges]);

	return (
		<pixiCustomViewport
			events={app.renderer.events}
			drag
			pinch
			wheel={{ trackpadPinch: true, percent: 2 }}
			passiveWheel={false}
		>
			<pixiContainer>
				{/* Pass 1: Roads */}
				{Array.from(edgePairs.values()).map(({ canonical, reverse }) => {
					const startNode = nodeMap.get(canonical.from);
					const endNode = nodeMap.get(canonical.to);
					if (!startNode || !endNode) return null;
					return (
						<Road
							key={`road-${canonical.id}`}
							canonicalEdge={canonical}
							reverseEdge={reverse}
							startNode={startNode}
							endNode={endNode}
						/>
					);
				})}

				{/* Pass 2: Intersections */}
				{data.nodes.map((node) => (
					<Intersection key={`node-${node.id}`} node={node} />
				))}

				{/* Pass 3: Traffic Light Indicators */}
				{data.edges.map((edge, index) => {
					const startNode = nodeMap.get(edge.from);
					const endNode = nodeMap.get(edge.to);
					if (!startNode || !endNode) return null;
					if (!endNode.has_traffic_light) return null;
					const tl = trafficLights.get(endNode.id);
					const isGreen = tl ? tl.green_road_ids.includes(edge.id) : false;
					return (
						<TrafficLightIndicator
							key={`tli-${edge.id}-${index}`}
							start={startNode}
							end={endNode}
							edge={edge}
							isGreen={isGreen}
						/>
					);
				})}

				{/* Pass 4: Vehicles */}
				{vehicles.map((vehicle) => (
					<Vehicle key={`vehicle-${vehicle.id}`} data={vehicle} />
				))}
			</pixiContainer>
		</pixiCustomViewport>
	);
}
