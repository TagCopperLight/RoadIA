'use client';

import { useState, useEffect, useCallback } from 'react';
import { MapNode, MapEdge } from './map/types';

interface PropertiesPanelProps {
	selectedNode: MapNode | null;
	selectedEdge: MapEdge | null;
	onUpdateNode: (id: number, kind: string, name: string) => void;
	onDeleteNode: (id: number) => void;
	onUpdateEdge: (id: number, lane_count: number, speed_limit: number, is_blocked: boolean, can_overtake: boolean) => void;
	onDeleteEdge: (id: number) => void;
}

export default function PropertiesPanel({
	selectedNode,
	selectedEdge,
	onUpdateNode,
	onDeleteNode,
	onUpdateEdge,
	onDeleteEdge,
}: PropertiesPanelProps) {
	// Node form state
	const [nodeName, setNodeName] = useState('');
	const [nodeKind, setNodeKind] = useState<'Intersection' | 'Habitation' | 'Workplace'>('Intersection');

	// Edge form state
	const [laneCount, setLaneCount] = useState(1);
	const [speedLimit, setSpeedLimit] = useState(40);
	const [isBlocked, setIsBlocked] = useState(false);
	const [canOvertake, setCanOvertake] = useState(false);

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
			setIsBlocked(selectedEdge.is_blocked ?? false);
			setCanOvertake(selectedEdge.can_overtake ?? false);
		}
	}, [selectedEdge]);

	const handleNodeCommit = useCallback(() => {
		if (selectedNode) {
			onUpdateNode(selectedNode.id, nodeKind, nodeName);
		}
	}, [selectedNode, nodeKind, nodeName, onUpdateNode]);

	const handleEdgeCommit = useCallback(() => {
		if (selectedEdge) {
			onUpdateEdge(selectedEdge.id, laneCount, speedLimit, isBlocked, canOvertake);
		}
	}, [selectedEdge, laneCount, speedLimit, isBlocked, canOvertake, onUpdateEdge]);

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
							className={inputClass}
							value={nodeName}
							onChange={e => setNodeName(e.target.value)}
							onBlur={handleNodeCommit}
							onKeyDown={e => e.key === 'Enter' && handleNodeCommit()}
						/>
					</div>

					<div>
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

					<div className="flex items-center gap-[8px]">
						<input
							type="checkbox"
							id="is_blocked"
							checked={isBlocked}
							onChange={e => {
								setIsBlocked(e.target.checked);
								// Commit immediately for checkboxes
								if (selectedEdge) {
									onUpdateEdge(selectedEdge.id, laneCount, speedLimit, e.target.checked, canOvertake);
								}
							}}
							className="accent-yellow-400"
						/>
						<label htmlFor="is_blocked" className="text-[13px]">Blocked</label>
					</div>

					<div className="flex items-center gap-[8px]">
						<input
							type="checkbox"
							id="can_overtake"
							checked={canOvertake}
							onChange={e => {
								setCanOvertake(e.target.checked);
								if (selectedEdge) {
									onUpdateEdge(selectedEdge.id, laneCount, speedLimit, isBlocked, e.target.checked);
								}
							}}
							className="accent-yellow-400"
						/>
						<label htmlFor="can_overtake" className="text-[13px]">Can Overtake</label>
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
