'use client';

import Image from "next/image";
import { useCallback, useState } from 'react';
import { usePacket } from '@/app/websocket/websocket';
import { PixiApp } from './map/PixiApp';
import { MapData, VehicleData, ScoreData, TrafficLightData } from './map/types';

export default function MapComponent() {
	const [container, setContainer] = useState<HTMLDivElement | null>(null);
	const [mapData, setMapData] = useState<MapData | null>(null);
	const [vehicles, setVehicles] = useState<VehicleData[]>([]);
	const [score, setScore] = useState<ScoreData | null>(null);
	const [showScore, setShowScore] = useState(false);
	const prevVehiclesRef = useRef<Record<number, VehicleData>>({});
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

	useWebSocket("score", (data) => {
		setScore(data as ScoreData);
		setShowScore(true);
	})

	return (
		<div ref={onRefChange} className="w-full h-full rounded-[10px] overflow-hidden relative">
			{container && <PixiApp resizeTo={container} mapData={mapData} vehicles={vehicles} trafficLights={trafficLights} />}
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
