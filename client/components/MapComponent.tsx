'use client';

import Image from "next/image";
import { useCallback, useState } from 'react';
import { usePacket } from '@/app/websocket/websocket';
import { PixiApp } from './map/PixiApp';
import { MapData, VehicleData, TrafficLightData } from './map/types';

export default function MapComponent() {
	const [container, setContainer] = useState<HTMLDivElement | null>(null);
	const [mapData, setMapData] = useState<MapData | null>(null);
	const [vehicles, setVehicles] = useState<VehicleData[]>([]);
	const [trafficLights, setTrafficLights] = useState<Map<number, TrafficLightData>>(new Map());

	const onRefChange = useCallback((node: HTMLDivElement) => {
		setContainer(node);
	}, []);

	usePacket("map", (data) => {
		console.log("Received map data:", data);
		setMapData(data as MapData);
	});

	usePacket("vehicleUpdate", (data) => {
		const update = data as { vehicles?: VehicleData[], traffic_lights?: TrafficLightData[] };
        if (update && Array.isArray(update.vehicles)) {
			setVehicles(update.vehicles as VehicleData[]);
        }

		if (update && Array.isArray(update.traffic_lights)) {
			setTrafficLights(prev => {
				const next = new Map<number, TrafficLightData>();
				(update.traffic_lights as TrafficLightData[]).forEach(tl => next.set(tl.id, tl));
				// Skip re-render if green road sets haven't changed
				const changed = [...next.entries()].some(([k, v]) =>
					prev.get(k)?.green_road_ids.join() !== v.green_road_ids.join()
				);
				return changed ? next : prev;
			});
		}
	});

	return (
		<div ref={onRefChange} className="w-full h-full rounded-[10px] overflow-hidden relative">
			{container && <PixiApp resizeTo={container} mapData={mapData} vehicles={vehicles} trafficLights={trafficLights} />}
			<div className="absolute bottom-[15px] right-[15px] bg-white p-1 rounded-[10px] shadow-md group cursor-pointer">
				<Image src="/map/man.png" alt="Orange man" width={35} height={35} className="transition-transform duration-200 group-hover:-rotate-12" />
			</div>
		</div>
	);
}
