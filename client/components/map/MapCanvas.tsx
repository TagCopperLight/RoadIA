import { useMemo } from 'react';
import { useApplication } from '@pixi/react';
import { FederatedPointerEvent } from 'pixi.js';
import { MapData, MapEdge, TrafficLightData } from './types';
import { AppMode, EditTool, SelectedElement } from '../EditModeContext';
import { Road } from './elements/Road';
import { Intersection } from './elements/Intersection';
import { VehicleLayer } from './VehicleLayer';
import { TrafficLightIndicator } from './elements/TrafficLightIndicator';

interface MapCanvasProps {
	data: MapData;
	trafficLights: Map<number, TrafficLightData>;
	roadDensity: Map<number, number>;
	densityView: boolean;
	showIntersections: boolean;
	mode: AppMode;
	editTool: EditTool;
	selectedElement: SelectedElement;
	pendingRoadFrom: number | null;
	onSelectNode: (id: number) => void;
	onSelectRoad: (canonicalId: number, reverseId?: number) => void;
	onAddNode: (x: number, y: number) => void;
	onAddRoad: (nodeId: number) => void;
	onWaypointNodeClick: (nodeId: number) => void;
}

export function MapCanvas({
	data,
	trafficLights,
	roadDensity,
	densityView,
	showIntersections,
	mode,
	editTool,
	selectedElement,
	pendingRoadFrom,
	onSelectNode,
	onSelectRoad,
	onAddNode,
	onAddRoad,
	onWaypointNodeClick,
}: MapCanvasProps) {
	const { app } = useApplication();

	const nodeMap = useMemo(
		() => new Map(data.nodes.map(n => [n.id, n])),
		[data.nodes]
	);

	const edgePairs = useMemo(() => {
		const map = new Map<string, { canonical: MapEdge; reverse?: MapEdge }>();
		for (const edge of data.edges) {
			const key = `${Math.min(edge.from, edge.to)}-${Math.max(edge.from, edge.to)}`;
			const entry = map.get(key);
			if (!entry) {
				map.set(key, { canonical: edge });
			} else if (edge.from === entry.canonical.to) {
				entry.reverse = edge;
			}
		}
		return map;
	}, [data.edges]);

	// Background overlay handles addNode clicks and move-tool drag tracking
	const handleBackgroundTap = (e: FederatedPointerEvent) => {
		if (mode !== 'edit' || editTool !== 'addNode') return;
		const local = e.getLocalPosition(e.currentTarget);
		onAddNode(local.x, local.y);
	};

	const isEditMode = mode === 'edit';
	const backgroundActive = isEditMode && editTool === 'addNode';

	const staticMapElements = useMemo(() => {
		return (
			<>
				{/* Pass 1: Roads */}
				{Array.from(edgePairs.values()).map(({ canonical, reverse }) => {
					const startNode = nodeMap.get(canonical.from);
					const endNode = nodeMap.get(canonical.to);
					if (!startNode || !endNode) return null;
					const isSelected = selectedElement?.type === 'road' && selectedElement.canonicalId === canonical.id;
					return (
						<Road
							key={`road-${canonical.id}`}
							canonicalEdge={canonical}
							reverseEdge={reverse}
							startNode={startNode}
							endNode={endNode}
							isSelected={isSelected}
							isEditMode={isEditMode}
							densityView={densityView}
							speedRatio={densityView ? roadDensity.get(canonical.id) : undefined}
							reverseSpeedRatio={densityView && reverse ? roadDensity.get(reverse.id) : undefined}
							onSelect={isEditMode && editTool === 'select'
								? () => onSelectRoad(canonical.id, reverse?.id)
								: undefined}
						/>
					);
				})}

				{/* Pass 2: Intersections */}
				{data.nodes.map((node) => {
					if (!isEditMode && !showIntersections && node.kind === 'Intersection') return null;
					const isSelected = selectedElement?.type === 'node' && selectedElement.id === node.id;
					const isPendingFrom = pendingRoadFrom === node.id;
					return (
						<Intersection
							key={`node-${node.id}`}
							node={node}
							isSelected={isSelected}
							isEditMode={isEditMode}
							isPendingFrom={isPendingFrom}
							onSelect={isEditMode && editTool === 'select'
								? () => onSelectNode(node.id)
								: undefined}
							onAddRoad={isEditMode && editTool === 'addRoad'
								? () => onAddRoad(node.id)
								: undefined}
							onWaypointClick={isEditMode && (editTool === 'waypoints' || editTool === 'bus')
								? () => onWaypointNodeClick(node.id)
								: undefined}
						/>
					);
				})}

				{/* Pass 3: Traffic Light Indicators */}
				{data.edges.map((edge, index) => {
					const startNode = nodeMap.get(edge.from);
					const endNode = nodeMap.get(edge.to);
					if (!startNode || !endNode) return null;
					if (!endNode.has_traffic_light) return null;
					const tl = trafficLights.get(endNode.id);
					const isGreen = tl ? tl.green_road_ids.includes(edge.id) : false;
					return (
						<TrafficLightIndicator
							key={`tli-${edge.id}-${index}`}
							start={startNode}
							end={endNode}
							edge={edge}
							isGreen={isGreen}
						/>
					);
				})}
			</>
		);
	}, [edgePairs, data.nodes, data.edges, nodeMap, trafficLights, selectedElement, isEditMode, editTool, onSelectRoad, onSelectNode, onAddRoad, onWaypointNodeClick, pendingRoadFrom, roadDensity, densityView, showIntersections]);

	return (
		<pixiCustomViewport
			events={app.renderer.events}
			drag
			pinch
			wheel={{ trackpadPinch: true, percent: 2 }}
			passiveWheel={false}
		>
			<pixiContainer>
        {/* Background hit area — addNode clicks + move-tool drag tracking */}
				<pixiGraphics
					draw={(g) => {
						g.clear();
						g.setFillStyle({ color: 0x000000, alpha: 0 });
						g.rect(-100000, -100000, 200000, 200000);
						g.fill();
					}}
					eventMode={backgroundActive ? 'static' : 'none'}
					onPointerTap={handleBackgroundTap}
				/>
				{staticMapElements}

				{/* Pass 4: Vehicles animated directly by the Pixi ticker */}
				<VehicleLayer />
			</pixiContainer>
		</pixiCustomViewport>
	);
}
