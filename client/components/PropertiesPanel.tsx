'use client';

import { useState, useEffect, useCallback } from 'react';
import { MapNode, MapEdge } from './map/types';

interface PropertiesPanelProps {
	nodes: MapNode[];
	selectedNode: MapNode | null;
	selectedEdge: MapEdge | null;
	onUpdateNode: (id: number, kind: string, name: string) => void;
	onDeleteNode: (id: number) => void;
	onUpdateEdge: (id: number, lane_count: number, speed_limit: number, intersection_type?: string) => void;
	onDeleteEdge: (id: number) => void;
}

export default function PropertiesPanel({
	nodes,
	selectedNode,
	selectedEdge,
	onUpdateNode,
	onDeleteNode,
	onUpdateEdge,
	onDeleteEdge,
}: PropertiesPanelProps) {
	// Node form state
	const [nodeName, setNodeName] = useState('');
	const [nodeKind, setNodeKind] = useState<'Intersection' | 'Habitation' | 'Workplace' | 'RoundAbout' | 'TrafficLight'>('Intersection');
	const [nameError, setNameError] = useState<string | null>(null);

	// Edge form state
	const [laneCount, setLaneCount] = useState(1);
	const [speedLimit, setSpeedLimit] = useState(40);
	const [intersectionType, setIntersectionType] = useState<'Priority' | 'Yield' | 'Stop'>('Priority');

	// Sync form state when selection changes
	useEffect(() => {
		if (selectedNode) {
			setNodeName(selectedNode.name);
			setNodeKind(selectedNode.kind);
		}
	}, [selectedNode]);

	useEffect(() => {
		if (selectedEdge) {
			setLaneCount(selectedEdge.lane_count);
			setSpeedLimit(selectedEdge.speed_limit ?? 40);
			setIntersectionType(selectedEdge.intersection_type ?? 'Priority');
		}
	}, [selectedEdge]);

	const validateNodeName = (name: string): string | null => {
		if (!name.trim()) {
			return 'Node name cannot be empty';
		}
		const isDuplicate = nodes.some(n => n.id !== selectedNode?.id && n.name.toLowerCase() === name.toLowerCase());
		if (isDuplicate) {
			return 'A node with this name already exists';
		}
		return null;
	};

	const handleNodeCommit = useCallback(() => {
		if (selectedNode) {
			const error = validateNodeName(nodeName);
			if (error) {
				setNameError(error);
				return;
			}
			setNameError(null);
			onUpdateNode(selectedNode.id, nodeKind, nodeName);
		}
	}, [selectedNode, nodeKind, nodeName, onUpdateNode, nodes]);

	const handleEdgeCommit = useCallback(() => {
		if (selectedEdge) {
			onUpdateEdge(selectedEdge.id, laneCount, speedLimit, intersectionType);
		}
	}, [selectedEdge, laneCount, speedLimit, intersectionType, onUpdateEdge]);

	const labelClass = 'text-[12px] text-neutral-400 mb-[2px]';
	const inputClass = 'bg-neutral-700 text-white text-[13px] rounded-[6px] px-[8px] py-[4px] w-full outline-none focus:ring-1 focus:ring-yellow-400';

	return (
		<div className="absolute top-[15px] right-[15px] w-[260px] bg-neutral-900 rounded-[12px] p-[14px] shadow-xl text-white flex flex-col gap-[10px]">
			{selectedNode && (
				<>
					<p className="text-[14px] font-semibold text-neutral-200">Node Properties</p>

					<div>
						<p className={labelClass}>Name</p>
						<input
						className={`${inputClass} ${nameError ? 'ring-1 ring-red-500' : ''}`}
						value={nodeName}
						onChange={e => {
							setNodeName(e.target.value);
							const error = validateNodeName(e.target.value);
							setNameError(error);
						}}
						onBlur={handleNodeCommit}
						onKeyDown={e => e.key === 'Enter' && handleNodeCommit()}
					/>
					{nameError && <p className="text-[11px] text-red-400 mt-[4px]">{nameError}</p>}
						<p className={labelClass}>Kind</p>
						<select
							className={inputClass}
							value={nodeKind}
							onChange={e => {
								setNodeKind(e.target.value as typeof nodeKind);
							}}
							onBlur={handleNodeCommit}
						>
							<option value="Intersection">Intersection</option>
							<option value="Habitation">Habitation</option>
							<option value="Workplace">Workplace</option>
							<option value="RoundAbout">RoundAbout</option>
							<option value="TrafficLight">TrafficLight</option>
						</select>
					</div>

					<button
						className="mt-[4px] bg-red-600 hover:bg-red-500 text-white text-[13px] font-medium py-[5px] rounded-[6px] cursor-pointer transition-colors"
						onClick={() => onDeleteNode(selectedNode.id)}
					>
						Delete Node
					</button>
				</>
			)}

			{selectedEdge && (
				<>
					<p className="text-[14px] font-semibold text-neutral-200">Road Properties</p>

					<div>
						<p className={labelClass}>Lanes</p>
						<input
							type="number"
							min={1}
							max={6}
							className={inputClass}
							value={laneCount}
							onChange={e => setLaneCount(Number(e.target.value))}
							onBlur={handleEdgeCommit}
							onKeyDown={e => e.key === 'Enter' && handleEdgeCommit()}
						/>
					</div>

					<div>
						<p className={labelClass}>Speed limit (m/s)</p>
						<input
							type="number"
							min={1}
							max={42}
							className={inputClass}
							value={speedLimit}
							onChange={e => setSpeedLimit(Number(e.target.value))}
							onBlur={handleEdgeCommit}
							onKeyDown={e => e.key === 'Enter' && handleEdgeCommit()}
						/>
					</div>

					<div>
						<p className={labelClass}>Type</p>
						<select
							className={inputClass}
							value={intersectionType}
							onChange={e => {
								setIntersectionType(e.target.value as 'Priority' | 'Yield' | 'Stop');
							}}
							onBlur={handleEdgeCommit}
						>
							<option value="Priority">Priority</option>
							<option value="Yield">Yield</option>
							<option value="Stop">Stop</option>
						</select>
					</div>

					<button
						className="mt-[4px] bg-red-600 hover:bg-red-500 text-white text-[13px] font-medium py-[5px] rounded-[6px] cursor-pointer transition-colors"
						onClick={() => onDeleteEdge(selectedEdge.id)}
					>
						Delete Road
					</button>
				</>
			)}
		</div>
	);
}
