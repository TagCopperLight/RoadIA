import { Application, extend, PixiReactElementProps } from '@pixi/react';
import { Container, Graphics, Sprite, Text } from 'pixi.js';
import { CustomViewport } from './CustomViewport';
import { MapCanvas } from './MapCanvas';
import { MapData, VehicleData } from './types';
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
}

export function PixiApp({ resizeTo, mapData, vehicles }: AppProps) {
	const [isInitialized, setIsInitialized] = useState(false);
	const handleInit = useCallback(() => setIsInitialized(true), []);

	return (
		<Application onInit={handleInit} background={0xC1D9B7} resizeTo={resizeTo}>
			{isInitialized && mapData && <MapCanvas data={mapData} vehicles={vehicles} />}
		</Application>
	);
}
