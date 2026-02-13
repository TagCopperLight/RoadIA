'use client';

import Image from "next/image";
import { useCallback, useState, useEffect, useRef } from 'react';
import { sendConnectionToken, useWebSocket } from '@/app/websocket/websocket';
import { PixiApp } from './map/PixiApp';
import { MapData, VehicleData } from './map/types';

interface MapComponentProps {
	uuid: string;
}

export default function MapComponent({ uuid }: MapComponentProps) {
	const [container, setContainer] = useState<HTMLDivElement | null>(null);
	const [mapData, setMapData] = useState<MapData | null>(null);
	const [vehicles, setVehicles] = useState<VehicleData[]>([]);
	const prevVehiclesRef = useRef<Record<number, VehicleData>>({});

	const onRefChange = useCallback((node: HTMLDivElement) => {
		setContainer(node);
	}, []);

	useEffect(() => {
		sendConnectionToken("auth-token");
	}, []);

	useWebSocket("map", (data) => {
		console.log("Received map data:", data);
		setMapData(data as MapData);
	});


	useWebSocket("vehicleUpdate", (data: any) => {
        if (data && Array.isArray(data.vehicles)) {
			const newVehicles = data.vehicles as VehicleData[];
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
                    // Initial heading if unknown, maybe 0 or undefined
                    vehicle.heading = undefined;
                    vehicle.speed = 0;
                }
				return vehicle;
			});

			// Update ref for next frame
			const newPrevVehicles: Record<number, VehicleData> = {};
			processedVehicles.forEach(v => {
				newPrevVehicles[v.id] = v;
			});
			prevVehiclesRef.current = newPrevVehicles;

		    setVehicles(processedVehicles);
        }
	});

	return (
		<div ref={onRefChange} className="w-full h-full rounded-[10px] overflow-hidden relative">
			{container && <PixiApp resizeTo={container} mapData={mapData} vehicles={vehicles} />}
			<div className="absolute bottom-[15px] right-[15px] bg-white p-1 rounded-[10px] shadow-md group cursor-pointer">
				<Image src="/map/man.png" alt="Orange man" width={35} height={35} className="transition-transform duration-200 group-hover:-rotate-12" />
			</div>
		</div>
	);
}
