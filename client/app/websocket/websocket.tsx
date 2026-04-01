'use client';

import { createContext, useContext, useEffect, useRef, useState } from 'react';
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

        this.socket = new WebSocket(this.url);

        this.socket.onopen = () => {
            console.log(`[WS] Connected to ${this.url}`);
        };

        this.socket.onmessage = (event: MessageEvent) => {
            try {
                const { id, data } = JSON.parse(event.data);
                this.listeners.get(id)?.forEach(fn => fn(data));
            } catch (e) {
                console.error('[WS] Parse error:', e);
            }
        };

        this.socket.onclose = () => {
            console.log('[WS] Disconnected');
            if (this.shouldReconnect) {
                this.reconnectTimeout = setTimeout(() => this.open(), 3000);
            }
        };

        this.socket.onerror = () => {
            this.socket?.close();
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
    const [ws, setWs] = useState<WebSocketClient | null>(null);

    useEffect(() => {
        const token = sessionStorage.getItem('sim_token') ?? '';
        const wsUrl = process.env.NEXT_PUBLIC_API_URL!.replace(/^http/, 'ws');
        const client = new WebSocketClient(
            `${wsUrl}/ws?uuid=${encodeURIComponent(uuid)}&token=${encodeURIComponent(token)}`
        );
        client.connect();
        setWs(client);
        return () => client.close();
    }, [uuid]);

    return <WsContext.Provider value={ws}>{children}</WsContext.Provider>;
}

export function useWs(): WebSocketClient | null {
    return useContext(WsContext);
}

export function usePacket(packetID: string, callback: Listener) {
    const ws = useContext(WsContext);
    const callbackRef = useRef(callback);
    callbackRef.current = callback;

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
