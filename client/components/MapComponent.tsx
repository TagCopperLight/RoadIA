'use client';

import { Application, extend, PixiReactElementProps, useApplication } from '@pixi/react';
import { IViewportOptions, Viewport } from 'pixi-viewport';
import { Container, Graphics, Sprite, Text } from 'pixi.js';
import { useCallback, useState, type RefObject } from 'react';

class CustomViewport extends Viewport {
  constructor(
    options: IViewportOptions & {
      decelerate?: boolean;
      drag?: boolean;
      pinch?: boolean;
      wheel?: boolean;
    }
  ) {
    const { decelerate, drag, pinch, wheel, ...rest } = options;
    super(rest);
    if (decelerate) this.decelerate();
    if (drag) this.drag();
    if (pinch) this.pinch();
    if (wheel) this.wheel();
  }
}

declare module "@pixi/react" {
  interface PixiElements {
    pixiCustomViewport: PixiReactElementProps<typeof CustomViewport>;
  }
}

extend({ Container, Graphics, Sprite, Text, CustomViewport});

function Map() {
    const { app } = useApplication();
    
    return (
        <pixiCustomViewport
            events={app.renderer.events}
            drag
            pinch
            wheel
        >
            <pixiContainer>
                <pixiGraphics draw={(graphics) => {
                    graphics.clear();
                    graphics.setFillStyle({ color: 'red' });
                    graphics.rect(0, 0, 100, 100);
                    graphics.rect(app.screen.width-100, app.screen.height-100, 100, 100);
                    graphics.fill();
                }}/>
            </pixiContainer>
        </pixiCustomViewport>
    );
}


interface AppProps {
    resizeTo: RefObject<HTMLElement> |  HTMLElement;
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

    return (
        <div ref={onRefChange} className="w-full h-full rounded-[10px] overflow-hidden">
            {container && <App resizeTo={container} />}
        </div>
    );
}
