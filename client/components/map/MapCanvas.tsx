import { useApplication } from '@pixi/react';
import { MapData, VehicleData } from './types';
import { Road } from './elements/Road';
import { Intersection } from './elements/Intersection';
import { Vehicle } from './elements/Vehicle';

export function MapCanvas({ data, vehicles }: { data: MapData, vehicles: VehicleData[] }) {
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
				{data.edges.map((edge, index) => {
					const startNode = data.nodes.find(n => n.id === edge.from);
					const endNode = data.nodes.find(n => n.id === edge.to);
					if (!startNode || !endNode) return null;
					return <Road key={`road-${edge.id}-${index}`} start={startNode} end={endNode} />;
				})}
				{data.nodes.map((node) => (
					<Intersection key={`node-${node.id}`} node={node} />
				))}
                {vehicles.map((vehicle) => (
                    <Vehicle key={`vehicle-${vehicle.id}`} data={vehicle} />
                ))}
			</pixiContainer>
		</pixiCustomViewport>
	);
}
