'use client';

import Image from "next/image";
import { useCallback, useState, useRef } from 'react';
import { usePacket } from '@/app/websocket/websocket';
import { PixiApp } from './map/PixiApp';
import { MapData, VehicleData, TrafficLightData } from './map/types';

export default function MapComponent() {
	const [container, setContainer] = useState<HTMLDivElement | null>(null);
	const [mapData, setMapData] = useState<MapData | null>(null);
	const [vehicles, setVehicles] = useState<VehicleData[]>([]);
	const [trafficLights, setTrafficLights] = useState<Map<number, TrafficLightData>>(new Map());
	const prevVehiclesRef = useRef<Record<number, VehicleData>>({});

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
			const newVehicles = update.vehicles as VehicleData[];
			const processedVehicles = newVehicles.map(vehicle => {
				const prevVehicle = prevVehiclesRef.current[vehicle.id];

				if (prevVehicle) {
					const dx = vehicle.x - prevVehicle.x;
					const dy = vehicle.y - prevVehicle.y;
					const dist = Math.sqrt(dx * dx + dy * dy);

					if (dist > 0.01) {
						vehicle.heading = Math.atan2(dy, dx);
						vehicle.speed = dist;
					} else {
						vehicle.heading = prevVehicle.heading;
						vehicle.speed = 0;
					}
				} else {
                    vehicle.heading = undefined;
                    vehicle.speed = 0;
                }
				return vehicle;
			});

			const newPrevVehicles: Record<number, VehicleData> = {};
			processedVehicles.forEach(v => {
				newPrevVehicles[v.id] = v;
			});
			prevVehiclesRef.current = newPrevVehicles;

		    setVehicles(processedVehicles);
        }

		if (update && Array.isArray(update.traffic_lights)) {
			const tlMap = new Map<number, TrafficLightData>();
			(update.traffic_lights as TrafficLightData[]).forEach(tl => {
				tlMap.set(tl.id, tl);
			});
			setTrafficLights(tlMap);
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
