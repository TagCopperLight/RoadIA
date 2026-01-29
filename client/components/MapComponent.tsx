'use client';

import Image from "next/image";
import { Application, extend, PixiReactElementProps, useApplication } from '@pixi/react';
import { IViewportOptions, Viewport, IWheelOptions } from 'pixi-viewport';
import { Container, Graphics, Sprite, Text } from 'pixi.js';
import { useCallback, useState, useEffect, type RefObject } from 'react';
import { sendConnectionToken, useWebSocket } from '@/app/websocket/websocket';

interface MapNode {
	id: number;
	kind: "Intersection" | "Habitation" | "Workplace";
	name: string;
	x: number;
	y: number;
}

interface MapEdge {
	from: number;
	id: number;
	lane_count: number;
	length: number;
	to: number;
}

interface MapData {
	nodes: MapNode[];
	edges: MapEdge[];
}

class CustomViewport extends Viewport {
	constructor(
		options: IViewportOptions & {
			decelerate?: boolean;
			drag?: boolean;
			pinch?: boolean;
			wheel?: boolean | IWheelOptions;
		}
	) {
		const { decelerate, drag, pinch, wheel, ...rest } = options;
		super(rest);
		if (decelerate) this.decelerate();
		if (drag) this.drag();
		if (pinch) this.pinch();
		if (wheel) {
			if (typeof wheel === 'boolean') {
				this.wheel();
			} else {
				this.wheel(wheel);
			}
		}
	}
}

declare module "@pixi/react" {
	interface PixiElements {
		pixiCustomViewport: PixiReactElementProps<typeof CustomViewport>;
	}
}

extend({ Container, Graphics, Sprite, Text, CustomViewport });

function Road({ start, end }: { start: MapNode, end: MapNode }) {
	const width = 15;
	return (
		<pixiGraphics draw={(graphics) => {
			graphics.clear();

			const dx = end.x - start.x;
			const dy = end.y - start.y;
			const length = Math.sqrt(dx * dx + dy * dy);
			const angle = Math.atan2(dy, dx);

			graphics.position.set(start.x, start.y);
			graphics.rotation = angle;

			graphics.setFillStyle({ color: 'gray' });
			graphics.rect(0, -width / 2, length, width);
			graphics.fill();

			graphics.setStrokeStyle({ color: 'white' });
			graphics.moveTo(0, 0);
			graphics.lineTo(length, 0);
			graphics.stroke();
		}} />
	);
}

function Intersection({ node }: { node: MapNode }) {
	return (
		<pixiGraphics draw={(graphics) => {
			graphics.clear();
			graphics.position.set(node.x, node.y);
			graphics.setFillStyle({ color: node.kind === 'Habitation' ? 'blue' : node.kind === 'Workplace' ? 'red' : 'lightgray' });
			graphics.circle(0, 0, 10);
			graphics.fill();
		}} />
	);
}

function Map({ data }: { data: MapData }) {
	const { app } = useApplication();

	return (
		<pixiCustomViewport
			events={app.renderer.events}
			drag
			pinch
			wheel={{ trackpadPinch: true, percent: 2 }}
			passiveWheel={false}
		>
			<pixiContainer>
				{data.edges.map((edge, index) => {
					const startNode = data.nodes.find(n => n.id === edge.from);
					const endNode = data.nodes.find(n => n.id === edge.to);
					if (!startNode || !endNode) return null;
					return <Road key={`road-${edge.id}-${index}`} start={startNode} end={endNode} />;
				})}
				{data.nodes.map((node) => (
					<Intersection key={`node-${node.id}`} node={node} />
				))}
			</pixiContainer>
		</pixiCustomViewport>
	);
}


interface AppProps {
	resizeTo: RefObject<HTMLElement> | HTMLElement;
	mapData: MapData | null;
}

function App({ resizeTo, mapData }: AppProps) {
	const [isInitialized, setIsInitialized] = useState(false);
	const handleInit = useCallback(() => setIsInitialized(true), []);

	return (
		<Application onInit={handleInit} background={0xC1D9B7} resizeTo={resizeTo}>
			{isInitialized && mapData && <Map data={mapData} />}
		</Application>
	);
}

interface MapComponentProps {
	uuid: string;
}

export default function MapComponent({ uuid }: MapComponentProps) {
	const [container, setContainer] = useState<HTMLDivElement | null>(null);
	const [mapData, setMapData] = useState<MapData | null>(null);

	const onRefChange = useCallback((node: HTMLDivElement) => {
		setContainer(node);
	}, []);

	useEffect(() => {
		sendConnectionToken("auth-token");
	}, []);

	useWebSocket("map", (data) => {
		console.log("Received map data:", data);
		setMapData(data as MapData);
	});

	return (
		<div ref={onRefChange} className="w-full h-full rounded-[10px] overflow-hidden relative">
			{container && <App resizeTo={container} mapData={mapData} />}
			<div className="absolute bottom-[15px] right-[15px] bg-white p-1 rounded-[10px] shadow-md group cursor-pointer">
				<Image src="/map/man.png" alt="Orange man" width={35} height={35} className="transition-transform duration-200 group-hover:-rotate-12" />
			</div>
		</div>
	);

}
