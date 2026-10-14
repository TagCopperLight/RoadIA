import { Application, extend, PixiReactElementProps, useApplication } from '@pixi/react';
import { Container, Graphics, Sprite, Text } from 'pixi.js';
import { CustomViewport } from './CustomViewport';
import { MapCanvas } from './MapCanvas';
import { MapData, TrafficLightData } from './types';
import { AppMode, EditTool, SelectedElement } from '../EditModeContext';
import { RefObject, useCallback, useEffect, useState } from 'react';
import { useTheme } from '../ThemeContext';

extend({ Container, Graphics, Sprite, Text, CustomViewport });

declare module "@pixi/react" {
	interface PixiElements {
		pixiCustomViewport: PixiReactElementProps<typeof CustomViewport>;
	}
}

interface AppProps {
	resizeTo: RefObject<HTMLElement> | HTMLElement;
	mapData: MapData | null;
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

const BACKGROUND_LIGHT = 0xC1D9B7;
const BACKGROUND_DARK = 0x3A4636;

// `background` on <Application> is only read at init, so theme changes are applied to the live renderer.
function ThemedBackground({ color }: { color: number }) {
	const { app } = useApplication();
	useEffect(() => {
		app?.renderer?.background.color.setValue(color);
	}, [app, color]);
	return null;
}

export function PixiApp({
	resizeTo, mapData, trafficLights,
	roadDensity, densityView, showIntersections,
	mode, editTool, selectedElement, pendingRoadFrom,
	onSelectNode, onSelectRoad, onAddNode, onAddRoad, onWaypointNodeClick,
}: AppProps) {
	const [isInitialized, setIsInitialized] = useState(false);
	const handleInit = useCallback(() => setIsInitialized(true), []);
	const { theme } = useTheme();
	const background = theme === 'dark' ? BACKGROUND_DARK : BACKGROUND_LIGHT;

	return (
		<Application onInit={handleInit} background={background} resizeTo={resizeTo} antialias={true}>
			{isInitialized && <ThemedBackground color={background} />}
			{isInitialized && mapData && (
				<MapCanvas
					data={mapData}
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
					onWaypointNodeClick={onWaypointNodeClick}
				/>
			)}
		</Application>
	);
}
