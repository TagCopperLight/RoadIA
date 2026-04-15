'use client';

import Image from "next/image";
import { useCallback, useEffect, useRef, useState } from 'react';
import { usePacket, useWs } from '@/app/websocket/websocket';
import { useEditMode } from './EditModeContext';
import { PixiApp } from './map/PixiApp';
import { MapData, VehicleData, ScoreData, TrafficLightData } from './map/types';
import ScoreModal from './ScoreModal';
import PropertiesPanel from './PropertiesPanel';
import BudgetHUD from './BudgetHUD';
import { WaypointPanel } from './WaypointPanel';
import { calculateCost, estimateRoadCost, estimateNodeCost, MAX_BUDGET } from './map/budget';

interface VehicleInfo {
	id: number;
	origin_node_id: number;
	dest_node_id: number;
	vehicle_type: string;
}

interface WaypointPanelHandle {
	onNodeClick: (nodeId: number) => void;
	getSelectedVehicleId: () => number | null;
	getPendingWaypoints: () => number[];
}

export default function MapComponent() {
	const [container, setContainer] = useState<HTMLDivElement | null>(null);
	const [mapData, setMapData] = useState<MapData | null>(null);
	const [vehicles, setVehicles] = useState<VehicleData[]>([]);
	const [vehicleList, setVehicleList] = useState<VehicleInfo[]>([]);
	const [score, setScore] = useState<ScoreData | null>(null);
	const [showScore, setShowScore] = useState(false);
	const [trafficLights, setTrafficLights] = useState<Map<number, TrafficLightData>>(new Map());
	const [editError, setEditError] = useState<string | null>(null);
	const waypointPanelRef = useRef<WaypointPanelHandle | null>(null);

	const ws = useWs();
	const {
		mode, editTool, selectedElement, pendingRoadFrom, simState,
		setSelectedElement, setPendingRoadFrom, setEditTool, simulationResetAt,
	} = useEditMode();

	const simStateRef = useRef(simState);
	const modeRef = useRef(mode);
	useEffect(() => {
		simStateRef.current = simState;
		modeRef.current = mode;
	});

	// Refs for auto-selecting newly created nodes
	const pendingNewNodeRef = useRef(false);
	const prevNodeIdsRef = useRef<Set<number>>(new Set());

	// Refs for auto-selecting newly created roads
	const pendingNewRoadRef = useRef(false);
	const prevEdgeIdsRef = useRef<Set<number>>(new Set());
	const lastAddRoadFromRef = useRef<number | null>(null);
	const lastAddRoadToRef = useRef<number | null>(null);

	const onRefChange = useCallback((node: HTMLDivElement) => {
		setContainer(node);
	}, []);

	usePacket("map", (data) => {
		setMapData(data as MapData);
	});

	usePacket("vehicleUpdate", (data) => {
		if (simStateRef.current === 'stopped' || modeRef.current === 'edit') return;
		const update = data as { vehicles?: VehicleData[], traffic_lights?: TrafficLightData[] };
		if (update && Array.isArray(update.vehicles)) {
			setVehicles(update.vehicles as VehicleData[]);
		}
		if (update && Array.isArray(update.traffic_lights)) {
			setTrafficLights(prev => {
				const next = new Map<number, TrafficLightData>();
				(update.traffic_lights as TrafficLightData[]).forEach(tl => next.set(tl.id, tl));
				const changed = [...next.entries()].some(([k, v]) =>
					prev.get(k)?.green_road_ids.join() !== v.green_road_ids.join()
				);
				return changed ? next : prev;
			});
		}
	});

	usePacket("vehicleList", (data) => {
		const list = data as { vehicles: VehicleInfo[] };
		if (list && Array.isArray(list.vehicles)) {
			setVehicleList(list.vehicles);
		}
	});

	usePacket("score", (data) => {
		setScore(data as ScoreData);
		setShowScore(true);
	});

	usePacket("mapEdit", (data) => {
		const result = data as { success: boolean; error?: string; nodes: MapData['nodes']; edges: MapData['edges'] };
		if (result.success) {
			setMapData({ nodes: result.nodes, edges: result.edges });
			setPendingRoadFrom(null);

			// Auto-select newly created node
			if (pendingNewNodeRef.current) {
				pendingNewNodeRef.current = false;
				const prevIds = prevNodeIdsRef.current;
				const newNode = result.nodes.find((n: MapData['nodes'][number]) => !prevIds.has(n.id));
				if (newNode) {
					setSelectedElement({ type: 'node', id: newNode.id });
					setEditTool('select');
				}
			}

			// Auto-select newly created road
			if (pendingNewRoadRef.current) {
				pendingNewRoadRef.current = false;
				const prevIds = prevEdgeIdsRef.current;
				const fromId = lastAddRoadFromRef.current;
				const toId = lastAddRoadToRef.current;

				if (fromId !== null && toId !== null) {
					// Find the canonical edge (from_id → to_id)
					const canonicalEdge = result.edges.find(
						(e: MapData['edges'][number]) => e.from === fromId && e.to === toId && !prevIds.has(e.id)
					);
					// Find the reverse edge (to_id → from_id) if it exists
					const reverseEdge = result.edges.find(
						(e: MapData['edges'][number]) => e.from === toId && e.to === fromId && !prevIds.has(e.id)
					);

					if (canonicalEdge) {
						setSelectedElement({ type: 'road', canonicalId: canonicalEdge.id, reverseId: reverseEdge?.id });
						setEditTool('select');
					}
				}
			}

			// Resync road selection (handles one-way/two-way toggle and other road edits)
			if (selectedElement?.type === 'road') {
				const edges = result.edges as MapData['edges'];
				const can = edges.find((e: MapData['edges'][number]) => e.id === selectedElement.canonicalId);
				if (can) {
					const newRev = edges.find((e: MapData['edges'][number]) => e.from === can.to && e.to === can.from);
					if (newRev?.id !== selectedElement.reverseId) {
						setSelectedElement({ type: 'road', canonicalId: can.id, reverseId: newRev?.id });
					}
				}
			}
		} else {
			const msg = result.error ?? 'Unknown error';
			setEditError(msg);
			setTimeout(() => setEditError(null), 3000);
		}
	});

	// Dismiss error on click
	useEffect(() => {
		if (!editError) return;
		const t = setTimeout(() => setEditError(null), 3000);
		return () => clearTimeout(t);
	}, [editError]);



	// Clear vehicles when simulation is reset
	const [prevResetAt, setPrevResetAt] = useState(simulationResetAt);
	if (simulationResetAt !== prevResetAt) {
		setPrevResetAt(simulationResetAt);
		setVehicles([]);
	}

	const handleAddNode = useCallback((x: number, y: number) => {
		if (mapData) {
			if (calculateCost(mapData) + estimateNodeCost('Intersection') > MAX_BUDGET) {
				setEditError('Budget exceeded: not enough funds to add this intersection.');
				return;
			}
		}
		// Snapshot current node IDs before the add
		prevNodeIdsRef.current = new Set(mapData?.nodes.map(n => n.id) ?? []);
		pendingNewNodeRef.current = true;
		ws?.send('addNode', { x, y, kind: 'Intersection' });
	}, [ws, mapData]);

	const handleAddRoad = useCallback((nodeId: number) => {
		if (pendingRoadFrom === null) {
			setPendingRoadFrom(nodeId);
		} else if (pendingRoadFrom !== nodeId) {
			if (mapData) {
				const fromNode = mapData.nodes.find(n => n.id === pendingRoadFrom);
				const toNode   = mapData.nodes.find(n => n.id === nodeId);
				if (fromNode && toNode) {
					if (calculateCost(mapData) + estimateRoadCost(fromNode, toNode, 2) > MAX_BUDGET) {
						setEditError('Budget exceeded: not enough funds to build this road.');
						setPendingRoadFrom(null);
						return;
					}
				}
			}
			// Snapshot current edge IDs before the add
			prevEdgeIdsRef.current = new Set(mapData?.edges.map(e => e.id) ?? []);
			lastAddRoadFromRef.current = pendingRoadFrom;
			lastAddRoadToRef.current = nodeId;
			pendingNewRoadRef.current = true;
			ws?.send('addRoad', { from_id: pendingRoadFrom, to_id: nodeId, lane_count: 2, speed_limit: 13.9 });
			setPendingRoadFrom(null);
		}
	}, [ws, pendingRoadFrom, setPendingRoadFrom, mapData]);

	const handleSelectNode = useCallback((id: number) => {
		setSelectedElement({ type: 'node', id });
	}, [setSelectedElement]);

	const handleSelectRoad = useCallback((canonicalId: number, reverseId?: number) => {
		setSelectedElement({ type: 'road', canonicalId, reverseId });
	}, [setSelectedElement]);

	const handleWaypointNodeClick = useCallback((nodeId: number) => {
		// When in edit mode and a node is clicked, pass it to the waypoint panel
		if (waypointPanelRef.current) {
			waypointPanelRef.current.onNodeClick(nodeId);
		}
	}, []);

// In edit mode, show no vehicles
	const visibleVehicles = mode === 'edit' ? [] : vehicles;

	return (
		<div className="w-full h-full relative flex">
			<div ref={onRefChange} className="flex-1 rounded-[10px] overflow-hidden relative">
				{container && (
					<PixiApp
						resizeTo={container}
						mapData={mapData}
						vehicles={visibleVehicles}
						trafficLights={trafficLights}
						mode={mode}
						editTool={editTool}
						selectedElement={selectedElement}
						pendingRoadFrom={pendingRoadFrom}
						onSelectNode={handleSelectNode}
						onSelectRoad={handleSelectRoad}
						onAddNode={handleAddNode}
						onAddRoad={handleAddRoad}
					onWaypointNodeClick={handleWaypointNodeClick}
					allNodesMap={mapData ? new Map(mapData.nodes.map(n => [n.id, n])) : null}				/>
				)}
				<BudgetHUD mapData={mapData} />
				<div className="absolute bottom-[15px] right-[15px] bg-white p-1 rounded-[10px] shadow-md group cursor-pointer">
					<Image src="/map/man.png" alt="Orange man" width={35} height={35} className="transition-transform duration-200 group-hover:-rotate-12" />
				</div>

				{editError && (
					<div className="absolute top-[15px] left-1/2 -translate-x-1/2 bg-red-600 text-white text-sm px-4 py-2 rounded-lg shadow-lg">
						{editError}
					</div>
				)}

				{showScore && score && (
					<ScoreModal score={score} onClose={() => setShowScore(false)} />
				)}
			</div>

			{/* Properties panel sidebar */}
			{mode === 'edit' && selectedElement && mapData && (
				<PropertiesPanel
					selectedElement={selectedElement}
					mapData={mapData}
					onClose={() => setSelectedElement(null)}
					onSendPacket={(id, data) => ws?.send(id, data)}
				/>
			)}

			{/* Waypoint panel sidebar - visible when waypoints tool is selected */}
			{mode === 'edit' && editTool === 'waypoints' && (
				<div className="w-80 h-full flex flex-col">
					<WaypointPanel
						ref={waypointPanelRef}
						vehicles={vehicleList}
					/>
				</div>
			)}
		</div>
	);
}
