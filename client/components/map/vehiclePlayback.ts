import type { VehicleData, VehicleUpdatePacket } from './types';

interface Frame {
    time: number;
    vehicles: Map<number, VehicleData>;
}

// Render slightly behind the simulation so movement is between known snapshots,
// rather than repeatedly easing towards (and slowing down near) a moving target.
export class VehiclePlayback {
    private frames: Frame[] = [];
    private anchorTime = 0;
    private anchorWallTime = 0;
    private lastArrival = 0;
    private speed = 1;
    private delay = 0.1;
    private lastRenderTime = -Infinity;

    clear() {
        this.frames = [];
        this.lastRenderTime = -Infinity;
    }

    push(packet: VehicleUpdatePacket, now: number) {
        const speed = packet.playback_speed ?? 1;
        const interval = packet.snapshot_interval_s ?? 0.1;
        const latest = this.frames.at(-1);
        if (!latest || packet.simulation_time_s <= latest.time || speed !== this.speed
            || now - this.lastArrival > Math.max(1000, interval * 3000)) {
            this.clear();
            this.anchorTime = packet.simulation_time_s;
            this.anchorWallTime = now;
        }
        this.speed = speed;
        this.delay = Math.max(0.1, interval * 1.5) * speed;
        this.lastArrival = now;
        this.frames.push({
            time: packet.simulation_time_s,
            vehicles: new Map(packet.vehicles.map(v => [v.id, v])),
        });
        // Bound memory even while the tab is hidden and the ticker is suspended.
        if (this.frames.length > 120) this.frames.splice(0, this.frames.length - 120);
    }

    sample(now: number) {
        if (!this.frames.length) return null;
        const latest = this.frames[this.frames.length - 1];
        const time = Math.max(this.lastRenderTime, Math.min(latest.time,
            this.anchorTime + (now - this.anchorWallTime) / 1000 * this.speed - this.delay));
        this.lastRenderTime = time;
        while (this.frames.length > 1 && this.frames[1].time <= time) this.frames.shift();
        const from = this.frames[0];
        const to = this.frames[1] ?? from;
        const alpha = to === from ? 0 : Math.max(0, Math.min(1, (time - from.time) / (to.time - from.time)));
        return { from: from.vehicles, to: to.vehicles, alpha };
    }
}

export function interpolatePose(from: VehicleData, to: VehicleData, alpha: number) {
    const heading = from.heading ?? 0;
    const difference = (to.heading ?? heading) - heading;
    return {
        x: from.x + (to.x - from.x) * alpha,
        y: from.y + (to.y - from.y) * alpha,
        heading: heading + Math.atan2(Math.sin(difference), Math.cos(difference)) * alpha,
    };
}
