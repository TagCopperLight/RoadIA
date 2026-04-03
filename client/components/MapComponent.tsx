'use client';

import Image from "next/image";
import { useCallback, useState, useEffect, useRef } from 'react';
import { sendConnectionToken, useWebSocket } from '@/app/websocket/websocket';
import { PixiApp } from './map/PixiApp';
import { MapData, VehicleData, ScoreData } from './map/types';

interface MapComponentProps {
	uuid: string;
}

export default function MapComponent({ uuid }: MapComponentProps) {
	const [container, setContainer] = useState<HTMLDivElement | null>(null);
	const [mapData, setMapData] = useState<MapData | null>(null);
	const [vehicles, setVehicles] = useState<VehicleData[]>([]);
	const [score, setScore] = useState<ScoreData | null>(null);
	const [showScore, setShowScore] = useState(false);
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

	useWebSocket("score", (data) => {
		setScore(data as ScoreData);
		setShowScore(true);
	})

	return (
		<div ref={onRefChange} className="w-full h-full rounded-[10px] overflow-hidden relative">
			{container && <PixiApp resizeTo={container} mapData={mapData} vehicles={vehicles} />}
			<div className="absolute bottom-[15px] right-[15px] bg-white p-1 rounded-[10px] shadow-md group cursor-pointer">
				<Image src="/map/man.png" alt="Orange man" width={35} height={35} className="transition-transform duration-200 group-hover:-rotate-12" />
			</div>

			{showScore && score && (
				<div className="absolute inset-0 flex items-center justify-center bg-black/40 backdrop-blur-sm z-50">
					<div className="bg-white p-8 rounded-2xl shadow-2xl max-w-md w-full mx-4 transform transition-all animate-in fade-in zoom-in duration-300">
						<div className="flex justify-between items-start mb-6">
							<h2 className="text-3xl font-bold text-gray-800">Simulation Terminée</h2>
							<button 
								onClick={() => setShowScore(false)}
								className="text-gray-400 hover:text-gray-600 transition-colors cursor-pointer"
							>
								<svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
									<path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M6 18L18 6M6 6l12 12" />
								</svg>
							</button>
						</div>
						
						<div className="space-y-4">
							<div className="bg-gray-50 p-4 rounded-xl flex justify-between items-center">
								<span className="font-semibold text-lg">Score Final</span>
								<span className="text-3xl font-black text-gray-900">{score.score.toFixed(3)}</span>
							</div>
							
							<div className="grid grid-cols-2 gap-4">
								<div className="bg-gray-50 p-3 rounded-lg">
									<p className="text-xs text-gray-500 uppercase font-bold mb-1">Taux de réussite</p>
									<p className="text-xl font-bold text-gray-800">{(score.success_rate * 100).toFixed(0)}%</p>
								</div>
								<div className="bg-gray-50 p-3 rounded-lg">
									<p className="text-xs text-gray-500 uppercase font-bold mb-1">CO2 Émis</p>
									<p className="text-xl font-bold text-gray-800">{score.total_emitted_co2.toFixed(2)}kg</p>
								</div>
								<div className="bg-gray-50 p-3 rounded-lg">
									<p className="text-xs text-gray-500 uppercase font-bold mb-1">Temps total</p>
									<p className="text-xl font-bold text-gray-800">{score.total_trip_time.toFixed(0)}s</p>
								</div>
								<div className="bg-gray-50 p-3 rounded-lg">
									<p className="text-xs text-gray-500 uppercase font-bold mb-1">Distance parcourue</p>
									<p className="text-xl font-bold text-gray-800">{(score.total_distance_traveled / 1000).toFixed(2)}km</p>
								</div>
							</div>
						</div>

						<button 
							onClick={() => setShowScore(false)}
							className="mt-8 w-full bg-black hover:bg-neutral-800 text-white font-bold py-3 rounded-xl transition-colors cursor-pointer"
						>
							Fermer
						</button>
					</div>
				</div>
			)}
		</div>
	);
}
