'use client';

import { useState, useEffect, useRef } from 'react';
import MapComponent from './MapComponent';
import Toolbar from './Toolbar';
import { EditTool } from './map/types';
import { wsClient } from '@/app/websocket/websocket';

interface MapPageClientProps {
	uuid: string;
}

export default function MapPageClient({ uuid }: MapPageClientProps) {
	const [editMode, setEditMode] = useState(false);
	const [activeTool, setActiveTool] = useState<EditTool>('select');
	const [selectedNodeId, setSelectedNodeId] = useState<number | null>(null);
	const [selectedEdgeId, setSelectedEdgeId] = useState<number | null>(null);

	// Refs to latest values for use in keydown handler without stale closures.
	const selectedNodeIdRef = useRef(selectedNodeId);
	const selectedEdgeIdRef = useRef(selectedEdgeId);
	const editModeRef = useRef(editMode);
	selectedNodeIdRef.current = selectedNodeId;
	selectedEdgeIdRef.current = selectedEdgeId;
	editModeRef.current = editMode;

	// Delete/Backspace and Escape shortcuts.
	useEffect(() => {
		const handleKeyDown = (e: KeyboardEvent) => {
			if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;
			if (!editModeRef.current) return;

			if (e.key === 'Delete' || e.key === 'Backspace') {
				if (selectedNodeIdRef.current !== null) {
					wsClient.send('deleteNode', { id: selectedNodeIdRef.current });
					setSelectedNodeId(null);
				} else if (selectedEdgeIdRef.current !== null) {
					wsClient.send('deleteRoad', { id: selectedEdgeIdRef.current });
					setSelectedEdgeId(null);
				}
			}
			if (e.key === 'Escape') {
				setSelectedNodeId(null);
				setSelectedEdgeId(null);
			}
		};
		window.addEventListener('keydown', handleKeyDown);
		return () => window.removeEventListener('keydown', handleKeyDown);
	}, []);

	return (
		<>
			<Toolbar
				editMode={editMode}
				setEditMode={setEditMode}
				activeTool={activeTool}
				setActiveTool={setActiveTool}
				onClearSelection={() => {
					setSelectedNodeId(null);
					setSelectedEdgeId(null);
				}}
			/>
			<div className='flex w-full h-full pl-[15px] pr-[15px] pt-[15px] pb-[15px]'>
				<MapComponent
					uuid={uuid}
					editMode={editMode}
					activeTool={activeTool}
					selectedNodeId={selectedNodeId}
					setSelectedNodeId={setSelectedNodeId}
					selectedEdgeId={selectedEdgeId}
					setSelectedEdgeId={setSelectedEdgeId}
				/>
			</div>
		</>
	);
}
