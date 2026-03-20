import { useApplication } from '@pixi/react';
import { useRef, useState, useCallback, useEffect } from 'react';
import { FederatedPointerEvent } from 'pixi.js';
import { CustomViewport } from './CustomViewport';
import { MapData, VehicleData, EditTool, MapNode, MapEdge } from './types';
import { Road } from './elements/Road';
import { Intersection } from './elements/Intersection';
import { Vehicle } from './elements/Vehicle';

interface MapCanvasProps {
	data: MapData;
	vehicles: VehicleData[];
	editMode: boolean;
	activeTool: EditTool;
	selectedNodeId: number | null;
	setSelectedNodeId: (id: number | null) => void;
	selectedEdgeId: number | null;
	setSelectedEdgeId: (id: number | null) => void;
	sendPacket: (packetId: string, data: object) => void;
}

function hitTestNode(worldX: number, worldY: number, node: MapNode): boolean {
	const dx = worldX - node.x;
	const dy = worldY - node.y;
	return dx * dx + dy * dy <= 16 * 16; // radius 10 + glow ring 6
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
	return distSq <= 7.5 * 7.5; // road half-width
}

export function MapCanvas({
	data,
	vehicles,
	editMode,
	activeTool,
	selectedNodeId,
	setSelectedNodeId,
	selectedEdgeId,
	setSelectedEdgeId,
	sendPacket,
}: MapCanvasProps) {
	const { app } = useApplication();
	const viewportRef = useRef<CustomViewport | null>(null);

	// Add Road: source node waiting for destination.
	const [addRoadSource, setAddRoadSource] = useState<number | null>(null);
	const addRoadSourceRef = useRef<number | null>(null);
	const setAddRoadSourceSync = useCallback((id: number | null) => {
		addRoadSourceRef.current = id;
		setAddRoadSource(id);
	}, []);

	// Rubber-band pointer position (in world coords)
	const [pointerPos, setPointerPos] = useState<{ x: number; y: number } | null>(null);
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

	// Reset transient state when leaving edit mode or switching tools.
	useEffect(() => {
		if (!editMode) {
			setAddRoadSourceSync(null);
			setPointerPos(null);
			setDraggingNodeId(null);
			setDragPos(null);
		} else {
			setAddRoadSourceSync(null);
			setPointerPos(null);
		}
	}, [editMode, activeTool, setAddRoadSourceSync]);

	// All stage-level pointer and click handlers.
	// Using a single stage-level click handler with hit-testing avoids all
	// event-propagation ordering issues with pixi-viewport.
	useEffect(() => {
		if (!editMode) return;

		const onMove = (e: FederatedPointerEvent) => {
			const worldPos = toWorld(e.global.x, e.global.y);
			setPointerPos(worldPos);
			if (draggingNodeId !== null) {
				hasDraggedRef.current = true;
				setDragPos(worldPos);
			}
		};

		const onUp = () => {
			if (draggingNodeId !== null && hasDraggedRef.current && dragPos) {
				sendPacket('moveNode', { id: draggingNodeId, x: Math.round(dragPos.x), y: Math.round(dragPos.y) });
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
					} else if (source !== clickedNode.id) {
						sendPacket('addRoad', { from_id: source, to_id: clickedNode.id, lane_count: 1, speed_limit: 40.0 });
						setAddRoadSourceSync(null);
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
				sendPacket('addNode', { x: Math.round(worldPos.x), y: Math.round(worldPos.y), kind: 'Intersection', name: 'New Node' });
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
	}, [editMode, draggingNodeId, dragPos, sendPacket, app.stage, toWorld, setAddRoadSourceSync, setSelectedNodeId, setSelectedEdgeId]);

	// Make stage interactive so it receives pointer events.
	useEffect(() => {
		app.stage.eventMode = editMode ? 'static' : 'passive';
	}, [editMode, app.stage]);

	// Drag initiation: onPointerDown on the intersection identifies which node to drag.
	const handleNodePointerDown = useCallback((nodeId: number, e: FederatedPointerEvent) => {
		if (!editMode || activeTool !== 'select') return;
		e.stopPropagation(); // prevent viewport from starting a pan drag
		hasDraggedRef.current = false;
		setDraggingNodeId(nodeId);
		setDragPos(toWorld(e.global.x, e.global.y));
	}, [editMode, activeTool, toWorld]);

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
			drag={!editMode}
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
							editMode={editMode}
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
							editMode={editMode}
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
				{editMode && activeTool === 'addRoad' && sourceNode && pointerPos && (
					<pixiGraphics
						draw={(g) => {
							g.clear();
							const src = getNodePos(sourceNode);
							g.setStrokeStyle({ color: 0xffff00, width: 2, alpha: 0.8 });
							g.moveTo(src.x, src.y);
							g.lineTo(pointerPos.x, pointerPos.y);
							g.stroke();
						}}
					/>
				)}
			</pixiContainer>
		</pixiCustomViewport>
	);
}
