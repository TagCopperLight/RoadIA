'use client';

import Image from "next/image";
import { Application, extend, PixiReactElementProps, useApplication } from '@pixi/react';
import { IViewportOptions, Viewport, IWheelOptions } from 'pixi-viewport';
import { Container, Graphics, Sprite, Text } from 'pixi.js';
import { useCallback, useState, useEffect, type RefObject } from 'react';
import { sendConnectionToken, useWebSocket } from '@/app/websocket/websocket';

const intersections = [
	{ x: -400, y: 400 },
	{ x: -400, y: -400 },
	{ x: 400, y: -400 },
	{ x: 400, y: 400 },
];

const roads = [
	{ start: intersections[0], end: intersections[1] },
	{ start: intersections[1], end: intersections[2] },
	{ start: intersections[2], end: intersections[3] },
	{ start: intersections[3], end: intersections[0] },
	{ start: intersections[1], end: intersections[3] },
];

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

function Road({ start, end }: { start: { x: number, y: number }, end: { x: number, y: number } }) {
	const width = 40;
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

function Intersection({ x, y }: { x: number, y: number }) {
	return (
		<pixiGraphics draw={(graphics) => {
			graphics.clear();
			graphics.position.set(x, y);
			graphics.setFillStyle({ color: 'lightgray' });
			graphics.circle(0, 0, 20);
			graphics.fill();
		}} />
	);
}

function Map() {
	const { app } = useApplication();

	return (
		<pixiCustomViewport
			events={app.renderer.events}
			drag
			pinch
			wheel={{ trackpadPinch: true, percent: 2 }}
			passiveWheel={false}
		>
			<pixiContainer x={app.screen.width / 2} y={app.screen.height / 2}>
				{roads.map((road, index) => (
					<Road key={index} start={road.start} end={road.end} />
				))}
				{intersections.map((intersection, index) => (
					<Intersection key={index} x={intersection.x} y={intersection.y} />
				))}
			</pixiContainer>
		</pixiCustomViewport>
	);
}


interface AppProps {
	resizeTo: RefObject<HTMLElement> | HTMLElement;
}

function App({ resizeTo }: AppProps) {
	const [isInitialized, setIsInitialized] = useState(false);
	const handleInit = useCallback(() => setIsInitialized(true), []);

	return (
		<Application onInit={handleInit} background={0xC1D9B7} resizeTo={resizeTo}>
			{isInitialized && <Map />}
		</Application>
	);
}

interface MapComponentProps {
	uuid: string;
}

export default function MapComponent({ uuid }: MapComponentProps) {
	const [container, setContainer] = useState<HTMLDivElement | null>(null);
	const onRefChange = useCallback((node: HTMLDivElement) => {
		setContainer(node);
	}, []);

	useEffect(() => {
		sendConnectionToken("auth-token");
	}, []);

	useWebSocket("map", (data) => {
		console.log(data);
	});

	return (
		<div ref={onRefChange} className="w-full h-full rounded-[10px] overflow-hidden relative">
			{container && <App resizeTo={container} />}
			<div className="absolute bottom-[15px] right-[15px] bg-white p-1 rounded-[10px] shadow-md group cursor-pointer">
				<Image src="/map/man.png" alt="Orange man" width={35} height={35} className="transition-transform duration-200 group-hover:-rotate-12" />
			</div>
		</div>
	);

}
