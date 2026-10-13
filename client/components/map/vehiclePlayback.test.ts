import { describe, expect, it } from 'vitest';
import { interpolatePose, VehiclePlayback } from './vehiclePlayback';
import type { VehicleData, VehicleUpdatePacket } from './types';

const vehicle = (x: number, heading = 0): VehicleData => ({
    id: 1, x, y: 0, heading, state: 'Moving', kind: 'Car', commute_plan_id: null,
});
const packet = (time: number, x: number, speed = 1): VehicleUpdatePacket => ({
    vehicles: [vehicle(x)], traffic_lights: [], simulation_time_s: time,
    playback_speed: speed, snapshot_interval_s: 0.1 / speed,
});
const position = (playback: VehiclePlayback, now: number) => {
    const frame = playback.sample(now)!;
    return interpolatePose(frame.from.get(1)!, frame.to.get(1)!, frame.alpha).x;
};

describe('vehicle playback', () => {
    it.each([1, 3])('keeps constant-speed motion uniform at %sx despite packet jitter', speed => {
        const playback = new VehiclePlayback();
        for (let i = 0; i < 7; i++) {
            playback.push(packet(i * 0.1, i * 10, speed), i * 100 / speed + (i % 2 ? 7 : 0));
        }
        const start = 150 + 100 / speed;
        const samples = [0, 10, 20, 30].map(offset => position(playback, start + offset));
        for (let i = 1; i < samples.length; i++) expect(samples[i] - samples[i - 1]).toBeCloseTo(speed);
    });

    it('uses elapsed time consistently across frame rates', () => {
        const makePlayback = () => {
            const playback = new VehiclePlayback();
            for (let i = 0; i <= 10; i++) playback.push(packet(i * 0.1, i * 10), i * 100);
            return playback;
        };
        const fast = makePlayback();
        const slow = makePlayback();
        for (let now = 0; now < 650; now += 1000 / 144) fast.sample(now);
        for (let now = 0; now < 650; now += 1000 / 30) slow.sample(now);
        expect(position(fast, 650)).toBeCloseTo(position(slow, 650));
    });

    it('holds the last known position when packets stop, without overshooting', () => {
        const playback = new VehiclePlayback();
        playback.push(packet(0, 0), 0);
        playback.push(packet(0.1, 10), 100);
        expect(position(playback, 500)).toBe(10);
        expect(position(playback, 5000)).toBe(10);
    });

    it('reanchors after pause, reset and speed changes', () => {
        const playback = new VehiclePlayback();
        playback.push(packet(5, 50), 0);
        playback.push(packet(5.1, 60), 100);
        position(playback, 1000);
        playback.push(packet(5.2, 70), 5000);
        expect(position(playback, 5000)).toBe(70);
        playback.push(packet(0, 0), 5100);
        expect(position(playback, 5100)).toBe(0);
        playback.push(packet(0.1, 10, 3), 5200);
        expect(position(playback, 5200)).toBe(10);
        playback.clear();
        expect(playback.sample(5300)).toBeNull();
    });

    it('rotates across the angle wrap by the shortest path', () => {
        const pose = interpolatePose(vehicle(0, Math.PI - 0.1), vehicle(10, -Math.PI + 0.1), 0.5);
        expect(pose.heading).toBeCloseTo(Math.PI);
        expect(pose.x).toBe(5);
    });

    it('keeps lifecycle changes on the buffered timeline', () => {
        const playback = new VehiclePlayback();
        playback.push(packet(0, 0), 0);
        const arrived = packet(0.1, 10);
        arrived.vehicles[0].state = 'Arrived';
        playback.push(arrived, 100);
        expect(playback.sample(200)!.from.get(1)!.state).toBe('Moving');
        expect(playback.sample(251)!.from.get(1)!.state).toBe('Arrived');
    });
});
