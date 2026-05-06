import { Application, extend, PixiReactElementProps } from '@pixi/react';
import { Container, Graphics, Sprite, Text } from 'pixi.js';
import { CustomViewport } from './CustomViewport';
import { MapCanvas } from './MapCanvas';
import { MapData, VehicleData, TrafficLightData } from './types';
import { AppMode, EditTool, SelectedElement } from '../EditModeContext';
import { RefObject, useCallback, useState } from 'react';

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
}

export function PixiApp({
	resizeTo, mapData, vehicles, trafficLights,
	roadDensity, densityView, showIntersections,
	mode, editTool, selectedElement, pendingRoadFrom,
	onSelectNode, onSelectRoad, onAddNode, onAddRoad,
}: AppProps) {
	const [isInitialized, setIsInitialized] = useState(false);
	const handleInit = useCallback(() => setIsInitialized(true), []);

	return (
		<Application onInit={handleInit} background={0xC1D9B7} resizeTo={resizeTo} antialias={true}>
			{isInitialized && mapData && (
				<MapCanvas
					data={mapData}
					vehicles={vehicles}
					trafficLights={trafficLights}
					roadDensity={roadDensity}
					densityView={densityView}
					showIntersections={showIntersections}
					mode={mode}
					editTool={editTool}
					selectedElement={selectedElement}
					pendingRoadFrom={pendingRoadFrom}
					onSelectNode={onSelectNode}
					onSelectRoad={onSelectRoad}
					onAddNode={onAddNode}
					onAddRoad={onAddRoad}
				/>
			)}
		</Application>
	);
}
