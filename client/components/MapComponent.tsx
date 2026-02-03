'use client';

import Image from "next/image";
import { useCallback, useState, useEffect } from 'react';
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
		    setVehicles(data.vehicles);
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
