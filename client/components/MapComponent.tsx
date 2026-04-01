'use client';

import Image from "next/image";
import { useCallback, useState, useEffect, useRef } from 'react';
import { sendConnectionToken, useWebSocket, wsClient } from '@/app/websocket/websocket';
import { PixiApp } from './map/PixiApp';
import { MapData, VehicleData } from './map/types';
import PropertiesPanel from './PropertiesPanel';
import { useMapEditor } from '@/context/MapEditorContext';

interface MapComponentProps {
	uuid: string;
}

export default function MapComponent({ uuid }: MapComponentProps) {
	const { selectedNodeId, setSelectedNodeId, selectedEdgeId, setSelectedEdgeId, addToast } = useMapEditor();
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
		setMapData(data as MapData);
	});

	useWebSocket("mapEdit", useCallback((data: any) => {
		if (data.success) {
			setMapData({ nodes: data.nodes, edges: data.edges });
		} else {
			console.error("[MapEdit] Error:", data.error);
		}
	}, []));

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
	});

	const sendPacket = useCallback((packetId: string, data: object) => {
		wsClient.send(packetId, data);
	}, []);

	const selectedNode = mapData?.nodes.find(n => n.id === selectedNodeId) ?? null;
	const selectedEdge = mapData?.edges.find(e => e.id === selectedEdgeId) ?? null;

	return (
		<div ref={onRefChange} className="w-full h-full rounded-[10px] overflow-hidden relative">
			{container && (
				<PixiApp
					resizeTo={container}
					mapData={mapData}
					vehicles={vehicles}
					sendPacket={sendPacket}
					onUpdateEdge={(id: number, lane_count: number, speed_limit: number, is_blocked: boolean, can_overtake: boolean, intersection_type?: string) =>
						sendPacket('updateRoad', { id, lane_count, speed_limit, is_blocked, can_overtake, intersection_type })
					}
					onDeleteEdge={(id: number) => {
						sendPacket('deleteRoad', { id });
						setSelectedEdgeId(null);
					}}
				/>
			)}

			{(selectedNode || selectedEdge) && (
				<PropertiesPanel
					selectedNode={selectedNode}
					selectedEdge={selectedEdge}
					onUpdateNode={(id: number, kind: string, name: string) =>
						sendPacket('updateNode', { id, kind, name })
					}
					onDeleteNode={(id: number) => {
						sendPacket('deleteNode', { id });
						setSelectedNodeId(null);
					}}
					onUpdateEdge={(id: number, lane_count: number, speed_limit: number, is_blocked: boolean, can_overtake: boolean, intersection_type?: string) =>
						sendPacket('updateRoad', { id, lane_count, speed_limit, is_blocked, can_overtake, intersection_type })
					}
					onDeleteEdge={(id: number) => {
						sendPacket('deleteRoad', { id });
						setSelectedEdgeId(null);
					}}
				/>
			)}

			<div className="absolute bottom-[15px] right-[15px] bg-white p-1 rounded-[10px] shadow-md group cursor-pointer">
				<Image src="/map/man.png" alt="Orange man" width={35} height={35} className="transition-transform duration-200 group-hover:-rotate-12" />
			</div>
		</div>
	);
}
