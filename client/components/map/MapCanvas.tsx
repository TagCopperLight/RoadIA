import { useApplication } from '@pixi/react';
import { MapData, VehicleData, TrafficLightData } from './types';
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
				{data.edges.map((edge, index) => {
					const startNode = data.nodes.find(n => n.id === edge.from);
					const endNode = data.nodes.find(n => n.id === edge.to);
					if (!startNode || !endNode) return null;
					return <Road key={`road-${edge.id}-${index}`} start={startNode} end={endNode} />;
				})}

				{/* Pass 2: Traffic Light Indicators */}
				{data.edges.map((edge, index) => {
					const startNode = data.nodes.find(n => n.id === edge.from);
					const endNode = data.nodes.find(n => n.id === edge.to);
					if (!startNode || !endNode) return null;
					if (!endNode.has_traffic_light) return null;
					const tl = trafficLights.get(endNode.id);
					const isGreen = tl ? tl.green_road_ids.includes(edge.id) : false;
					return (
						<TrafficLightIndicator
							key={`tli-${edge.id}-${index}`}
							start={startNode}
							end={endNode}
							edgeId={edge.id}
							isGreen={isGreen}
						/>
					);
				})}

				{/* Pass 3: Intersections */}
				{data.nodes.map((node) => (
					<Intersection key={`node-${node.id}`} node={node} />
				))}

				{/* Pass 4: Vehicles */}
				{vehicles.map((vehicle) => (
					<Vehicle key={`vehicle-${vehicle.id}`} data={vehicle} />
				))}
			</pixiContainer>
		</pixiCustomViewport>
	);
}
