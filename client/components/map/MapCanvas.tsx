import { useApplication } from '@pixi/react';
import { useRef, useState, useCallback, useEffect } from 'react';
import { FederatedPointerEvent } from 'pixi.js';
import { CustomViewport } from './CustomViewport';
import { MapData, VehicleData, MapNode, MapEdge } from './types';
import { Road } from './elements/Road';
import { Intersection } from './elements/Intersection';
import { Vehicle } from './elements/Vehicle';
import { useMapEditor } from '@/context/MapEditorContext';
import { MAP_CONFIG } from '@/lib/constants';
import { ToastType } from '@/hooks/useToast';
import {
	validateNodeCreation,
	validateDifferentNodes,
	ValidationError,
} from '@/lib/validators';

interface MapCanvasProps {
	data: MapData;
	vehicles: VehicleData[];
	sendPacket: (packetId: string, data: object) => void;
	onToast: (message: string, type: ToastType, duration?: number) => void;
}

function hitTestNode(worldX: number, worldY: number, node: MapNode): boolean {
	const dx = worldX - node.x;
	const dy = worldY - node.y;
	return dx * dx + dy * dy <= MAP_CONFIG.NODE_HIT_RADIUS ** 2;
}

function hitTestEdge(worldX: number, worldY: number, edge: MapEdge, nodes: MapNode[]): boolean {
	const startNode = nodes.find(n => n.id === edge.from);
	const endNode = nodes.find(n => n.id === edge.to);
	if (!startNode || !endNode) return false;
	const dx = endNode.x - startNode.x;
	const dy = endNode.y - startNode.y;
	const lenSq = dx * dx + dy * dy;
	if (lenSq === 0) return false;
	const t = Math.max(0, Math.min(1, ((worldX - startNode.x) * dx + (worldY - startNode.y) * dy) / lenSq));
	const projX = startNode.x + t * dx;
	const projY = startNode.y + t * dy;
	const distSq = (worldX - projX) ** 2 + (worldY - projY) ** 2;
	return distSq <= MAP_CONFIG.ROAD_HIT_RADIUS ** 2;
}

export function MapCanvas({ data, vehicles, sendPacket, onToast }: MapCanvasProps) {
	const { activeTool, selectedNodeId, setSelectedNodeId, selectedEdgeId, setSelectedEdgeId } = useMapEditor();
	const { app } = useApplication();
	const viewportRef = useRef<CustomViewport | null>(null);

	// Pan tracking
	const panStartRef = useRef<{ x: number; y: number } | null>(null);
	const panLastRef = useRef<{ x: number; y: number } | null>(null);

	// Add Road: source node waiting for destination.
	const [addRoadSource, setAddRoadSource] = useState<number | null>(null);
	const addRoadSourceRef = useRef<number | null>(null);
	const setAddRoadSourceSync = useCallback((id: number | null) => {
		addRoadSourceRef.current = id;
		setAddRoadSource(id);
	}, []);

	// Pointer position tracking - use ref to avoid excessive re-renders
	const pointerPosRef = useRef<{ x: number; y: number } | null>(null);
	const [, setPointerPosState] = useState(0); // Force re-render when needed
	// Node dragging state
	const [draggingNodeId, setDraggingNodeId] = useState<number | null>(null);
	const [dragPos, setDragPos] = useState<{ x: number; y: number } | null>(null);
	const hasDraggedRef = useRef(false);

	// Always-fresh refs for use inside event handler closures.
	const activeToolRef = useRef(activeTool);
	activeToolRef.current = activeTool;
	const dataRef = useRef(data);
	dataRef.current = data;

	// Convert screen coordinates to viewport world coordinates.
	const toWorld = useCallback((screenX: number, screenY: number) => {
		if (viewportRef.current) {
			return viewportRef.current.toWorld(screenX, screenY);
		}
		return { x: screenX, y: screenY };
	}, []);

	// Reset transient state when switching tools.
	useEffect(() => {
		setAddRoadSourceSync(null);
		pointerPosRef.current = null;
		// Reset pan state when leaving pan tool
		if (activeTool !== 'pan') {
			panStartRef.current = null;
			panLastRef.current = null;
		}
	}, [activeTool, setAddRoadSourceSync]);

	// Manual pan handler (only active when tool is 'pan')
	useEffect(() => {
		if (activeTool !== 'pan') return;

		const onPointerDown = (e: FederatedPointerEvent) => {
			panStartRef.current = { x: e.global.x, y: e.global.y };
			panLastRef.current = { x: e.global.x, y: e.global.y };
		};

		const onPointerMove = (e: FederatedPointerEvent) => {
			if (!panStartRef.current || !panLastRef.current) return;

			const dx = e.global.x - panLastRef.current.x;
			const dy = e.global.y - panLastRef.current.y;

			if (viewportRef.current) {
				viewportRef.current.x += dx;
				viewportRef.current.y += dy;
			}

			panLastRef.current = { x: e.global.x, y: e.global.y };
		};

		const onPointerUp = () => {
			panStartRef.current = null;
			panLastRef.current = null;
		};

		app.stage.on('pointerdown', onPointerDown);
		app.stage.on('pointermove', onPointerMove);
		app.stage.on('pointerup', onPointerUp);
		app.stage.on('pointerupoutside', onPointerUp);

		return () => {
			app.stage.off('pointerdown', onPointerDown);
			app.stage.off('pointermove', onPointerMove);
			app.stage.off('pointerup', onPointerUp);
			app.stage.off('pointerupoutside', onPointerUp);
		};
	}, [activeTool, app.stage]);

	// All stage-level pointer and click handlers.
	// Using a single stage-level click handler with hit-testing avoids all
	// event-propagation ordering issues with pixi-viewport.
	useEffect(() => {
		const onMove = (e: FederatedPointerEvent) => {
			const worldPos = toWorld(e.global.x, e.global.y);
			pointerPosRef.current = worldPos;
			// Only trigger re-render when dragging (for preview purposes)
			if (activeTool === 'addNode' || (activeTool === 'addRoad' && addRoadSourceRef.current !== null)) {
				setPointerPosState(prev => prev + 1);
			}
			if (draggingNodeId !== null) {
				hasDraggedRef.current = true;
				setDragPos(worldPos);
			}
		};

		const onUp = () => {
			if (draggingNodeId !== null && hasDraggedRef.current && dragPos) {
				sendPacket('moveNode', { id: draggingNodeId, x: Math.round(dragPos.x), y: Math.round(dragPos.y) });
				onToast('Node moved', 'success');
			}
			setDraggingNodeId(null);
			setDragPos(null);
			hasDraggedRef.current = false;
		};

		const onClick = (e: FederatedPointerEvent) => {
			const tool = activeToolRef.current;
			const currentData = dataRef.current;
			const worldPos = toWorld(e.global.x, e.global.y);

			// Hit-test nodes first.
			const clickedNode = currentData.nodes.find(n => hitTestNode(worldPos.x, worldPos.y, n));
			if (clickedNode) {
				if (tool === 'select') {
					setSelectedNodeId(clickedNode.id);
					setSelectedEdgeId(null);
				} else if (tool === 'addRoad') {
					const source = addRoadSourceRef.current;
					if (source === null) {
						setAddRoadSourceSync(clickedNode.id);
						onToast('Select destination node', 'info');
					} else if (source !== clickedNode.id) {
						try {
							validateDifferentNodes(source, clickedNode.id);
							sendPacket('addRoad', { from_id: source, to_id: clickedNode.id, lane_count: MAP_CONFIG.DEFAULT_LANE_COUNT, speed_limit: MAP_CONFIG.DEFAULT_SPEED_LIMIT });
							onToast('Road added', 'success');
							setAddRoadSourceSync(null);
						} catch (err) {
							if (err instanceof ValidationError) {
								onToast(err.message, 'error');
							}
						}
					} else {
						onToast('Cannot create road to the same node', 'error');
					}
				}
				return;
			}

			// Hit-test edges (select tool only).
			if (tool === 'select') {
				const clickedEdge = currentData.edges.find(edge =>
					hitTestEdge(worldPos.x, worldPos.y, edge, currentData.nodes)
				);
				if (clickedEdge) {
					setSelectedEdgeId(clickedEdge.id);
					setSelectedNodeId(null);
					return;
				}
			}

			// Empty space click.
			if (tool === 'addNode') {
				try {
					validateNodeCreation(worldPos.x, worldPos.y, 'New Node', 'Intersection');
					sendPacket('addNode', { x: Math.round(worldPos.x), y: Math.round(worldPos.y), kind: 'Intersection', name: 'New Node' });
					onToast('Node added', 'success');
				} catch (err) {
					if (err instanceof ValidationError) {
						onToast(err.message, 'error');
					}
				}
			} else if (tool === 'select') {
				setSelectedNodeId(null);
				setSelectedEdgeId(null);
			} else if (tool === 'addRoad') {
				setAddRoadSourceSync(null);
			}
		};

		app.stage.on('pointermove', onMove);
		app.stage.on('pointerup', onUp);
		app.stage.on('pointerupoutside', onUp);
		app.stage.on('click', onClick);

		return () => {
			app.stage.off('pointermove', onMove);
			app.stage.off('pointerup', onUp);
			app.stage.off('pointerupoutside', onUp);
			app.stage.off('click', onClick);
		};
	}, [draggingNodeId, dragPos, sendPacket, app.stage, toWorld, setAddRoadSourceSync, setSelectedNodeId, setSelectedEdgeId, onToast]);

	// Make stage interactive so it receives pointer events.
	useEffect(() => {
		app.stage.eventMode = 'static';
	}, [app.stage]);

	// Drag initiation: onPointerDown on the intersection identifies which node to drag.
	const handleNodePointerDown = useCallback((nodeId: number, e: FederatedPointerEvent) => {
		if (activeTool !== 'select') return;
		e.stopPropagation(); // prevent viewport from starting a pan drag
		hasDraggedRef.current = false;
		setDraggingNodeId(nodeId);
		setDragPos(toWorld(e.global.x, e.global.y));
	}, [activeTool, toWorld]);

	// Source node coordinates for rubber-band line.
	const sourceNode = addRoadSource !== null ? data.nodes.find(n => n.id === addRoadSource) : null;

	// Effective node position: use drag position when dragging, otherwise actual position.
	const getNodePos = (node: MapNode) => {
		if (draggingNodeId === node.id && dragPos) return dragPos;
		return { x: node.x, y: node.y };
	};

	return (
		<pixiCustomViewport
			ref={viewportRef}
			events={app.renderer.events}
			pinch
			wheel={{ trackpadPinch: true, percent: 2 }}
			passiveWheel={false}
		>
			<pixiContainer>
				{data.edges.map((edge, index) => {
					const startNode = data.nodes.find(n => n.id === edge.from);
					const endNode = data.nodes.find(n => n.id === edge.to);
					if (!startNode || !endNode) return null;
					const startPos = getNodePos(startNode);
					const endPos = getNodePos(endNode);
					return (
						<Road
							key={`road-${edge.id}-${index}`}
							start={{ ...startNode, x: startPos.x, y: startPos.y }}
							end={{ ...endNode, x: endPos.x, y: endPos.y }}
							selected={selectedEdgeId === edge.id}
							activeTool={activeTool}
						/>
					);
				})}
				{data.nodes.map((node) => {
					const pos = getNodePos(node);
					return (
						<Intersection
							key={`node-${node.id}`}
							node={{ ...node, x: pos.x, y: pos.y }}
							selected={selectedNodeId === node.id}
							isAddRoadSource={addRoadSource === node.id}
							activeTool={activeTool}
							isDragging={draggingNodeId === node.id}
							onDragStart={(e) => handleNodePointerDown(node.id, e)}
						/>
					);
				})}
				{vehicles.map((vehicle) => (
					<Vehicle key={`vehicle-${vehicle.id}`} data={vehicle} />
				))}

				{/* Rubber-band line when adding a road */}
				{activeTool === 'addRoad' && sourceNode && pointerPosRef.current && (
					<pixiGraphics
						draw={(g) => {
							g.clear();
							const src = getNodePos(sourceNode);
						const currentPos = pointerPosRef.current;
						g.setStrokeStyle({
							color: MAP_CONFIG.RUBBER_BAND_COLOR,
							width: MAP_CONFIG.RUBBER_BAND_WIDTH,
							alpha: MAP_CONFIG.RUBBER_BAND_ALPHA,
						});
						g.moveTo(src.x, src.y);
						if (currentPos) {
							g.lineTo(currentPos.x, currentPos.y);
						}
						g.stroke();
					}}
				/>
			)}
			</pixiContainer>
		</pixiCustomViewport>
	);
}
