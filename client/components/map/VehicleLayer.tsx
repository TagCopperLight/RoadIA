import { useEffect, useRef } from 'react';
import { useApplication } from '@pixi/react';
import { Container, Graphics } from 'pixi.js';
import { usePacket } from '@/app/websocket/websocket';
import { useEditMode } from '../EditModeContext';
import type { VehicleUpdatePacket } from './types';
import { getVehicleStyle } from './elements/Vehicle';
import { interpolatePose, VehiclePlayback } from './vehiclePlayback';

export function VehicleLayer() {
    const { app } = useApplication();
    const { mode, simulationResetAt, simState } = useEditMode();
    const layer = useRef<Container>(null);
    const playback = useRef(new VehiclePlayback());

    usePacket('vehicleUpdate', data => {
        const packet = data as VehicleUpdatePacket;
        if (mode === 'simulation' && Array.isArray(packet?.vehicles)) {
            playback.current.push(packet, performance.now());
        }
    });

    useEffect(() => {
        playback.current.clear();
        const container = layer.current;
        for (const child of container?.removeChildren() ?? []) child.destroy();
    }, [mode, simulationResetAt]);

    useEffect(() => {
        // A resume starts a fresh wall-clock timeline, even after a short pause.
        if (simState === 'running') playback.current.clear();
    }, [simState]);

    useEffect(() => {
        const container = layer.current;
        if (!container) return;
        const graphics = new Map<number, { graphic: Graphics; styleKey: string }>();
        const tick = () => {
            // A mode/reset effect may have destroyed the previous graphics.
            for (const [id, entry] of graphics) {
                if (entry.graphic.destroyed) graphics.delete(id);
            }
            const frame = playback.current.sample(performance.now());
            if (!frame) return;
            for (const [id, vehicle] of frame.from) {
                if (vehicle.state !== 'Moving') continue;
                let entry = graphics.get(id);
                if (!entry) {
                    const graphic = new Graphics();
                    graphic.eventMode = 'none';
                    container.addChild(graphic);
                    entry = { graphic, styleKey: '' };
                    graphics.set(id, entry);
                }
                const style = getVehicleStyle(vehicle);
                const styleKey = `${style.color}:${style.w}:${style.h}`;
                if (entry.styleKey !== styleKey) {
                    entry.graphic.clear().rect(-style.w / 2, -style.h / 2, style.w, style.h).fill(style.color);
                    entry.styleKey = styleKey;
                }
                const next = frame.to.get(id);
                // Arrival coordinates are a node centre, not another driving pose.
                const pose = next?.state === 'Moving'
                    ? interpolatePose(vehicle, next, frame.alpha) : vehicle;
                entry.graphic.position.set(pose.x, pose.y);
                entry.graphic.rotation = pose.heading ?? 0;
            }
            for (const [id, entry] of graphics) {
                if (frame.from.get(id)?.state !== 'Moving') {
                    entry.graphic.destroy();
                    graphics.delete(id);
                }
            }
        };
        app.ticker.add(tick);
        return () => {
            app.ticker.remove(tick);
            for (const { graphic } of graphics.values()) graphic.destroy();
            graphics.clear();
        };
    }, [app]);

    return <pixiContainer ref={layer} eventMode="none" />;
}
