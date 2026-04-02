'use client';

import { createContext, useContext, useEffect, useMemo, useRef } from 'react';
import type { ReactNode } from 'react';

type Listener = (data: unknown) => void;

class WebSocketClient {
    private socket: WebSocket | null = null;
    private listeners: Map<string, Set<Listener>> = new Map();
    private url: string;
    private shouldReconnect: boolean = false;
    private reconnectTimeout: ReturnType<typeof setTimeout> | null = null;

    constructor(url: string) {
        this.url = url;
    }

    connect() {
        if (typeof window === 'undefined') return;
        this.shouldReconnect = true;
        this.open();
    }

    private open() {
        if (
            this.socket?.readyState === WebSocket.OPEN ||
            this.socket?.readyState === WebSocket.CONNECTING
        ) return;

        const ws = new WebSocket(this.url);
        this.socket = ws;

        ws.onopen = () => {
            if (this.socket !== ws) return;
            console.log(`[WS] Connected to ${this.url}`);
        };

        ws.onmessage = (event: MessageEvent) => {
            if (this.socket !== ws) return;
            try {
                const { id, data } = JSON.parse(event.data);
                this.listeners.get(id)?.forEach(fn => fn(data));
            } catch (e) {
                console.error('[WS] Parse error:', e);
            }
        };

        ws.onclose = (event: CloseEvent) => {
            if (this.socket !== ws) return;
            console.log(`[WS] Disconnected. Code: ${event.code}`);
            if (event.code === 4001) {
                console.warn('[WS] Unauthorized or map not found. Redirecting to home...');
                this.shouldReconnect = false;
                window.location.href = '/';
                return;
            }
            if (this.shouldReconnect) {
                this.reconnectTimeout = setTimeout(() => this.open(), 3000);
            }
        };

        ws.onerror = () => {
            if (this.socket !== ws) return;
            ws.close();
        };
    }

    close() {
        this.shouldReconnect = false;
        if (this.reconnectTimeout !== null) {
            clearTimeout(this.reconnectTimeout);
            this.reconnectTimeout = null;
        }
        this.socket?.close();
        this.socket = null;
    }

    send(packetID: string, data: unknown) {
        if (this.socket?.readyState === WebSocket.OPEN) {
            this.socket.send(JSON.stringify({ id: packetID, data }));
        }
    }

    on(packetID: string, listener: Listener) {
        if (!this.listeners.has(packetID)) this.listeners.set(packetID, new Set());
        this.listeners.get(packetID)!.add(listener);
    }

    off(packetID: string, listener: Listener) {
        this.listeners.get(packetID)?.delete(listener);
    }
}

const WsContext = createContext<WebSocketClient | null>(null);

export function WsProvider({ uuid, children }: { uuid: string; children: ReactNode }) {
    const client = useMemo(() => {
        const token = sessionStorage.getItem('sim_token') ?? '';
        const wsUrl = process.env.NEXT_PUBLIC_API_URL!.replace(/^http/, 'ws');
        return new WebSocketClient(
            `${wsUrl}/ws?uuid=${encodeURIComponent(uuid)}&token=${encodeURIComponent(token)}`
        );
    }, [uuid]);

    useEffect(() => {
        client.connect();
        return () => client.close();
    }, [client]);

    return <WsContext.Provider value={client}>{children}</WsContext.Provider>;
}

export function useWs(): WebSocketClient | null {
    return useContext(WsContext);
}

export function usePacket(packetID: string, callback: Listener) {
    const ws = useContext(WsContext);
    const callbackRef = useRef(callback);

    useEffect(() => {
        callbackRef.current = callback;
    });

    useEffect(() => {
        if (!ws) return;
        const stable: Listener = (data) => callbackRef.current(data);
        ws.on(packetID, stable);
        return () => ws.off(packetID, stable);
    }, [ws, packetID]);
}

export async function createSimulation(): Promise<{ uuid: string; token: string }> {
    const res = await fetch(`${process.env.NEXT_PUBLIC_API_URL}/api/simulations`, { method: 'POST' });
    if (!res.ok) throw new Error(`Failed to create simulation: ${res.status}`);
    return res.json();
}
