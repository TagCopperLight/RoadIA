import { Application, extend, PixiReactElementProps } from '@pixi/react';
import { Container, Graphics, Sprite, Text } from 'pixi.js';
import { CustomViewport } from './CustomViewport';
import { MapCanvas } from './MapCanvas';
import { MapData, VehicleData } from './types';
import { RefObject, useCallback, useState } from 'react';
import { useMapEditor } from '@/context/MapEditorContext';
import { MAP_CONFIG } from '@/lib/constants';

extend({ Container, Graphics, Sprite, Text, CustomViewport });

declare module "@pixi/react" {
	interface PixiElements {
		pixiCustomViewport: PixiReactElementProps<typeof CustomViewport>;
	}
}

interface AppProps {
	resizeTo: RefObject<HTMLElement> | HTMLElement;
	mapData: MapData | null;
	vehicles: VehicleData[];
	sendPacket: (packetId: string, data: object) => void;
	onUpdateEdge?: (id: number, lane_count: number, speed_limit: number, intersection_type?: string) => void;
	onDeleteEdge?: (id: number) => void;
}

export function PixiApp({ resizeTo, mapData, vehicles, sendPacket, onUpdateEdge, onDeleteEdge }: AppProps) {
	const { activeTool, selectedNodeId, setSelectedNodeId, selectedEdgeId, setSelectedEdgeId, addToast } = useMapEditor();
	const [isInitialized, setIsInitialized] = useState(false);
	const handleInit = useCallback(() => setIsInitialized(true), []);

	return (
		<Application onInit={handleInit} background={MAP_CONFIG.BACKGROUND_COLOR} resizeTo={resizeTo}>
			{isInitialized && mapData && (
				<MapCanvas
					data={mapData}
					vehicles={vehicles}
					sendPacket={sendPacket}
					onToast={addToast}
				/>
			)}
		</Application>
	);
}
