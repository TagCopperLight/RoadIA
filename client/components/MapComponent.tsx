'use client';

import Image from "next/image";
import { useCallback, useEffect, useRef, useState } from 'react';
import { usePacket, useWs } from '@/app/websocket/websocket';
import { useEditMode } from './EditModeContext';
import { PixiApp } from './map/PixiApp';
<<<<<<< HEAD
import { MapData, VehicleData, ScoreData, TrafficLightData, RoadMetricData, VehicleSummary } from './map/types';
=======
import { BusLine, MapData, VehicleData, ScoreData, TrafficLightData, RoadMetricData, VehicleSummary, VehicleUpdatePacket, ScoreProgressPacket } from './map/types';
>>>>>>> origin/dev
import ScoreModal from './ScoreModal';
import SettingsModal from './SettingsModal';
import PropertiesPanel from './PropertiesPanel';
import BudgetHUD from './BudgetHUD';
import WaypointPanel from './WaypointPanel';
<<<<<<< HEAD
=======
import BusPanel from './BusPanel';
>>>>>>> origin/dev
import Legend from './Legend';
import { calculateCost, estimateRoadCost, estimateNodeCost, DEFAULT_BUDGET_CONFIG } from './map/budget';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';

<<<<<<< HEAD
=======
const clampProgress = (value: number) => Math.max(0, Math.min(100, value));

>>>>>>> origin/dev
export default function MapComponent({ uuid }: { uuid: string }) {
	const [container, setContainer] = useState<HTMLDivElement | null>(null);
	const [mapData, setMapData] = useState<MapData | null>(null);
	const [vehicles, setVehicles] = useState<VehicleData[]>([]);
	const [vehicleSummaries, setVehicleSummaries] = useState<VehicleSummary[]>([]);
	const [score, setScore] = useState<ScoreData | null>(null);
	const [trafficLights, setTrafficLights] = useState<Map<number, TrafficLightData>>(new Map());
	const [roadDensity, setRoadDensity] = useState<Map<number, number>>(new Map());
<<<<<<< HEAD
	const [editError, setEditError] = useState<string | null>(null);
	const ws = useWs();
	const {
		mode, editTool, selectedElement, pendingRoadFrom, simState,
		setSelectedElement, setPendingRoadFrom, setEditTool, simulationResetAt,
		showScore, setShowScore, isScoringLoading, setIsScoringLoading,
=======
	const [simulationTime, setSimulationTime] = useState(0);
	const [editError, setEditError] = useState<string | null>(null);
	const ws = useWs();
	const {
		mode, editTool, selectedElement, pendingRoadFrom,
		setSelectedElement, setPendingRoadFrom, setEditTool, simulationResetAt,
		showScore, setShowScore, isScoringLoading, setIsScoringLoading,
		scoreProgress, setScoreProgress,
>>>>>>> origin/dev
		densityView, setDensityView, isDensityLoading, setIsDensityLoading,
		showIntersections,
		showSettings, setShowSettings, mapSettings, setMapSettings,
		waypointClickedNodeId, setWaypointClickedNodeId,
		waypointVehicleId, setWaypointVehicleId,
		pendingWaypoints, setPendingWaypoints,
<<<<<<< HEAD
=======
		busLines, setBusLines,
		busPendingStops, setBusPendingStops,
		busLineName, setBusLineName,
		busCreating, setBusCreating,
>>>>>>> origin/dev
	} = useEditMode();

	useEffect(() => {
		fetch(`${API_URL}/api/simulations/${uuid}/settings`)
			.then(r => r.json())
<<<<<<< HEAD
			.then(data => setMapSettings(data))
			.catch(() => {});
	}, [uuid, setMapSettings]);

	const simStateRef = useRef(simState);
	const modeRef = useRef(mode);
	useEffect(() => {
		simStateRef.current = simState;
=======
			.then(data => {
				setMapSettings(data);
				if (typeof data?.simulation_start_time === 'number') {
					setSimulationTime(data.simulation_start_time);
				}
			})
			.catch(() => {});
	}, [uuid, setMapSettings]);

	const modeRef = useRef(mode);
	useEffect(() => {
>>>>>>> origin/dev
		modeRef.current = mode;
	});

	// Request vehicle list when switching to waypoints tool
	useEffect(() => {
		if (editTool === 'waypoints') {
			ws?.send('requestVehicles', {});
		}
<<<<<<< HEAD
	}, [editTool, ws]);

=======
		if (editTool === 'bus') {
			ws?.send('requestBusLines', {});
		}
	}, [editTool, ws]);

	// Request the current vehicle snapshot when entering simulation mode or after a reset.
	useEffect(() => {
		if (mode === 'simulation') {
			ws?.send('requestVehicleUpdate', {});
		}
	}, [mode, simulationResetAt, ws]);

>>>>>>> origin/dev
	// Refs for auto-selecting newly created nodes
	const pendingNewNodeRef = useRef(false);
	const prevNodeIdsRef = useRef<Set<number>>(new Set());

	const onRefChange = useCallback((node: HTMLDivElement) => {
		setContainer(node);
	}, []);

	usePacket("map", (data) => {
		setMapData(data as MapData);
	});

	usePacket("vehicleUpdate", (data) => {
<<<<<<< HEAD
		if (simStateRef.current === 'stopped' || modeRef.current === 'edit') return;
		const update = data as { vehicles?: VehicleData[], traffic_lights?: TrafficLightData[] };
=======
		if (modeRef.current === 'edit') return;
		const update = data as VehicleUpdatePacket;
>>>>>>> origin/dev
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
<<<<<<< HEAD
=======
		if (typeof update?.simulation_time_s === 'number') {
			setSimulationTime(update.simulation_time_s);
		}
>>>>>>> origin/dev
	});

	usePacket("vehicleList", (data) => {
		const result = data as { vehicles: VehicleSummary[] };
		if (result && Array.isArray(result.vehicles)) {
			setVehicleSummaries(result.vehicles);
		}
	});

<<<<<<< HEAD
	usePacket("score", (data) => {
		setScore(data as ScoreData);
=======
	usePacket("busLineList", (data) => {
		const result = data as { bus_lines: BusLine[] };
		if (result && Array.isArray(result.bus_lines)) {
			setBusLines(result.bus_lines);
		}
	});

	usePacket("score", (data) => {
		setScore(data as ScoreData);
		setScoreProgress(100);
>>>>>>> origin/dev
		setIsScoringLoading(false);
		setShowScore(true);
	});

<<<<<<< HEAD
=======
	usePacket("scoreProgress", (data) => {
		const result = data as ScoreProgressPacket;
		if (typeof result?.progress === 'number') {
			const nextProgress = clampProgress(result.progress);
			setScoreProgress(prev => Math.max(prev ?? 0, nextProgress));
		}
	});

>>>>>>> origin/dev
	usePacket("densityMap", (data) => {
		const result = data as { edges: RoadMetricData[] };
		const next = new Map<number, number>();
		for (const m of result.edges) next.set(m.id, m.speed_ratio);
		setRoadDensity(next);
		setIsDensityLoading(false);
		setDensityView(true);
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
	useEffect(() => {
		if (simulationResetAt === 0) return;
		const t = setTimeout(() => {
			setVehicles([]);
			setRoadDensity(new Map());
			setDensityView(false);
			setIsDensityLoading(false);
<<<<<<< HEAD
		}, 0);
		return () => clearTimeout(t);
	}, [simulationResetAt]); // eslint-disable-line react-hooks/exhaustive-deps
=======
			setSimulationTime(mapSettings?.simulation_start_time ?? 0);
		}, 0);
		return () => clearTimeout(t);
	}, [simulationResetAt, mapSettings]); // eslint-disable-line react-hooks/exhaustive-deps

	const formatSimulationTime = (seconds: number) => {
		const totalSeconds = Math.max(0, Math.floor(seconds));
		const hours = Math.floor(totalSeconds / 3600);
		const minutes = Math.floor((totalSeconds % 3600) / 60);
		const secs = totalSeconds % 60;
		return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
	};
	const scoreProgressValue = scoreProgress === null ? null : clampProgress(scoreProgress);
	const scoreProgressLabel = scoreProgressValue === null ? null : Math.round(scoreProgressValue);
>>>>>>> origin/dev

	const handleAddNode = useCallback((x: number, y: number) => {
		if (mapData) {
			const cfg = mapSettings ?? DEFAULT_BUDGET_CONFIG;
			if (calculateCost(mapData, cfg) + estimateNodeCost('Intersection', cfg) > cfg.max_budget) {
				setEditError('Budget exceeded: not enough funds to add this intersection.');
				return;
			}
		}
		// Snapshot current node IDs before the add
		prevNodeIdsRef.current = new Set(mapData?.nodes.map(n => n.id) ?? []);
		pendingNewNodeRef.current = true;
		ws?.send('addNode', { x, y, kind: 'Intersection' });
	}, [ws, mapData, mapSettings]);

	const handleAddRoad = useCallback((nodeId: number) => {
		if (pendingRoadFrom === null) {
			setPendingRoadFrom(nodeId);
		} else if (pendingRoadFrom !== nodeId) {
			if (mapData) {
				const fromNode = mapData.nodes.find(n => n.id === pendingRoadFrom);
				const toNode   = mapData.nodes.find(n => n.id === nodeId);
				if (fromNode && toNode) {
					const cfg = mapSettings ?? DEFAULT_BUDGET_CONFIG;
					if (calculateCost(mapData, cfg) + estimateRoadCost(fromNode, toNode, 2, cfg) > cfg.max_budget) {
						setEditError('Budget exceeded: not enough funds to build this road.');
						setPendingRoadFrom(null);
						return;
					}
				}
			}
			ws?.send('addRoad', { from_id: pendingRoadFrom, to_id: nodeId, lane_count: 2, speed_limit: 13.9 });
			setPendingRoadFrom(null);
		}
	}, [ws, pendingRoadFrom, setPendingRoadFrom, mapData, mapSettings]);

	const handleSelectNode = useCallback((id: number) => {
		setSelectedElement({ type: 'node', id });
	}, [setSelectedElement]);

	const handleSelectRoad = useCallback((canonicalId: number, reverseId?: number) => {
		setSelectedElement({ type: 'road', canonicalId, reverseId });
	}, [setSelectedElement]);

	const handleWaypointNodeClick = useCallback((nodeId: number) => {
		if (waypointVehicleId !== null) {
			// Vehicle selected: add node as waypoint
			setPendingWaypoints(prev => [...prev, nodeId]);
		} else {
			// No vehicle selected yet: record clicked node for vehicle selection
			setWaypointClickedNodeId(nodeId);
		}
	}, [waypointVehicleId, setPendingWaypoints, setWaypointClickedNodeId]);

	const handleWaypointSelectVehicle = useCallback((vehicleId: number) => {
		const summary = vehicleSummaries.find(v => v.id === vehicleId);
		setWaypointVehicleId(vehicleId);
		setWaypointClickedNodeId(null);
		setPendingWaypoints(summary?.waypoint_ids ?? []);
	}, [vehicleSummaries, setWaypointVehicleId, setWaypointClickedNodeId, setPendingWaypoints]);

	const handleWaypointRemove = useCallback((index: number) => {
		setPendingWaypoints(prev => prev.filter((_, i) => i !== index));
	}, [setPendingWaypoints]);

	const handleWaypointApply = useCallback(() => {
		if (waypointVehicleId === null) return;
		ws?.send('addWaypoints', { vehicle_id: waypointVehicleId, node_ids: pendingWaypoints });
		setWaypointVehicleId(null);
		setWaypointClickedNodeId(null);
		setPendingWaypoints([]);
	}, [ws, waypointVehicleId, pendingWaypoints, setWaypointVehicleId, setWaypointClickedNodeId, setPendingWaypoints]);

	const handleWaypointCancel = useCallback(() => {
		setWaypointVehicleId(null);
		setWaypointClickedNodeId(null);
		setPendingWaypoints([]);
	}, [setWaypointVehicleId, setWaypointClickedNodeId, setPendingWaypoints]);

<<<<<<< HEAD
=======
	const handleBusStopClick = useCallback((nodeId: number) => {
		if (busCreating) {
			setBusPendingStops(prev => [...prev, nodeId]);
		}
	}, [busCreating, setBusPendingStops]);

	const handleCreateBusLine = useCallback((name: string, stops: number[]) => {
		ws?.send('createBusLine', { name, stop_node_ids: stops });
		setBusPendingStops([]);
		setBusLineName('');
		setBusCreating(false);
	}, [ws, setBusPendingStops, setBusLineName, setBusCreating]);

	const handleDeleteBusLine = useCallback((id: number) => {
		ws?.send('deleteBusLine', { bus_line_id: id });
	}, [ws]);

	const handleBusCancel = useCallback(() => {
		setBusPendingStops([]);
		setBusLineName('');
		setBusCreating(false);
	}, [setBusPendingStops, setBusLineName, setBusCreating]);

	// Route node clicks to correct handler based on active tool
	const handleSpecialModeNodeClick = useCallback((nodeId: number) => {
		if (editTool === 'waypoints') {
			handleWaypointNodeClick(nodeId);
		} else if (editTool === 'bus') {
			handleBusStopClick(nodeId);
		}
	}, [editTool, handleWaypointNodeClick, handleBusStopClick]);

>>>>>>> origin/dev
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
						roadDensity={roadDensity}
						densityView={densityView}
						showIntersections={showIntersections}
						mode={mode}
						editTool={editTool}
						selectedElement={selectedElement}
						pendingRoadFrom={pendingRoadFrom}
						onSelectNode={handleSelectNode}
						onSelectRoad={handleSelectRoad}
						onAddNode={handleAddNode}
						onAddRoad={handleAddRoad}
<<<<<<< HEAD
						onWaypointNodeClick={handleWaypointNodeClick}
=======
						onWaypointNodeClick={handleSpecialModeNodeClick}
>>>>>>> origin/dev
					/>
				)}
				<BudgetHUD mapData={mapData} />
				<Legend />
<<<<<<< HEAD
=======
				{mode === 'simulation' && (
					<div className="absolute top-[15px] left-1/2 -translate-x-1/2 bg-black/80 text-white rounded-full shadow-md px-4 py-2 z-30 font-mono text-sm tabular-nums pointer-events-none">
						{formatSimulationTime(simulationTime)}
					</div>
				)}
>>>>>>> origin/dev
				<div className="absolute bottom-[15px] right-[15px] bg-white p-1 rounded-[10px] shadow-md group cursor-pointer">
					<Image src="/map/man.png" alt="Orange man" width={35} height={35} className="transition-transform duration-200 group-hover:-rotate-12" />
				</div>

				{editError && (
					<div className="absolute top-[15px] left-1/2 -translate-x-1/2 bg-red-600 text-white text-sm px-4 py-2 rounded-lg shadow-lg">
						{editError}
					</div>
				)}

				{isScoringLoading && (
<<<<<<< HEAD
					<div className="absolute top-[15px] left-1/2 -translate-x-1/2 bg-white/90 backdrop-blur-sm border border-neutral-200 px-4 py-2 rounded-full shadow-lg flex items-center gap-3 z-40">
						<div className="w-4 h-4 border-2 border-neutral-300 border-t-neutral-800 rounded-full animate-spin" />
						<span className="text-sm font-medium text-neutral-800">Calcul du score...</span>
=======
					<div className="absolute top-[15px] left-1/2 -translate-x-1/2 bg-white/90 backdrop-blur-sm border border-neutral-200 px-4 py-3 rounded-2xl shadow-lg z-40 min-w-[280px]">
						<div className="flex items-center gap-3">
							<div className="w-4 h-4 border-2 border-neutral-300 border-t-neutral-800 rounded-full animate-spin" />
							<span className="text-sm font-medium text-neutral-800">Calcul du score...</span>
							{scoreProgressLabel !== null && (
								<span className="ml-auto text-sm font-semibold text-neutral-900 tabular-nums">
									{scoreProgressLabel}%
								</span>
							)}
						</div>
						{scoreProgressValue !== null && (
							<div className="mt-3 h-2 w-full rounded-full bg-neutral-200 overflow-hidden">
								<div
									className="h-full rounded-full bg-neutral-900 transition-[width] duration-150"
									style={{ width: `${scoreProgressValue}%` }}
								/>
							</div>
						)}
>>>>>>> origin/dev
					</div>
				)}

				{isDensityLoading && (
					<div className="absolute top-[15px] left-1/2 -translate-x-1/2 bg-white/90 backdrop-blur-sm border border-neutral-200 px-4 py-2 rounded-full shadow-lg flex items-center gap-3 z-40">
						<div className="w-4 h-4 border-2 border-neutral-300 border-t-neutral-800 rounded-full animate-spin" />
						<span className="text-sm font-medium text-neutral-800">Calcul de la densité...</span>
					</div>
				)}

				{showScore && score && (
<<<<<<< HEAD
					<ScoreModal score={score} onClose={() => setShowScore(false)} />
=======
					<ScoreModal score={score} onClose={() => { setShowScore(false); setScoreProgress(null); }} />
>>>>>>> origin/dev
				)}

				{showSettings && (
					<SettingsModal uuid={uuid} onClose={() => setShowSettings(false)} />
				)}
			</div>

			{/* Properties panel sidebar */}
<<<<<<< HEAD
			{mode === 'edit' && editTool !== 'waypoints' && selectedElement && mapData && (
=======
			{mode === 'edit' && editTool !== 'waypoints' && editTool !== 'bus' && selectedElement && mapData && (
>>>>>>> origin/dev
				<PropertiesPanel
					selectedElement={selectedElement}
					mapData={mapData}
					onClose={() => setSelectedElement(null)}
					onSendPacket={(id, data) => ws?.send(id, data)}
				/>
			)}

			{mode === 'edit' && editTool === 'waypoints' && (
				<WaypointPanel
					vehicleSummaries={vehicleSummaries}
					mapData={mapData}
					waypointNodeId={waypointClickedNodeId}
					waypointVehicleId={waypointVehicleId}
					pendingWaypoints={pendingWaypoints}
					onSelectVehicle={handleWaypointSelectVehicle}
					onRemoveWaypoint={handleWaypointRemove}
					onApply={handleWaypointApply}
					onCancel={handleWaypointCancel}
					onClose={() => setEditTool('select')}
				/>
			)}
<<<<<<< HEAD
=======

			{mode === 'edit' && editTool === 'bus' && (
				<BusPanel
					busLines={busLines}
					mapData={mapData}
					pendingStops={busPendingStops}
					lineName={busLineName}
					creating={busCreating}
					onNameChange={setBusLineName}
					onRemoveStop={(i) => setBusPendingStops(prev => prev.filter((_, idx) => idx !== i))}
					onCreate={handleCreateBusLine}
					onDelete={handleDeleteBusLine}
					onStartCreating={() => setBusCreating(true)}
					onCancel={handleBusCancel}
					onClose={() => { handleBusCancel(); setEditTool('select'); }}
				/>
			)}
>>>>>>> origin/dev
		</div>
	);
}
