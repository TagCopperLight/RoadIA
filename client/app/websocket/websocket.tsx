import { useEffect } from 'react';

type Listener = (data: any) => void;

class WebSocketClient {
    private socket: WebSocket | null = null;
    private listeners: Map<string, Listener[]> = new Map();
    private url: string;
    private reconnectInterval: number = 5000;
    private shouldReconnect: boolean = true;

    constructor(url: string) {
        this.url = url;
        this.connect();
    }

    private connect() {
        // Verify we are client-side
        if (typeof window === 'undefined') return;

        this.socket = new WebSocket(this.url);

        this.socket.onopen = () => {
            console.log(`[WebSocket] Connected to ${this.url}`);
        };

        this.socket.onmessage = (event: MessageEvent) => {
            try {
                const json = JSON.parse(event.data);
                const packetID = json.PacketID;
                const data = json.Data;
                this.dispatch(packetID, data);
            } catch (e) {
                console.error("[WebSocket] Failed to parse message:", e);
            }
        };

        this.socket.onclose = () => {
            console.log("[WebSocket] Disconnected");
            if (this.shouldReconnect) {
                setTimeout(() => this.connect(), this.reconnectInterval);
            }
        };

        this.socket.onerror = (error) => {
            console.error("[WebSocket] Error:", error);
            this.socket?.close();
        };
    }

    private dispatch(packetID: string, data: any) {
        const packetListeners = this.listeners.get(packetID);
        if (packetListeners) {
            packetListeners.forEach(callback => callback(data));
        } else {
            console.warn(`[WebSocket] No listener for packetID: "${packetID}"`);
        }
    }

    public on(packetID: string, callback: Listener) {
        if (!this.listeners.has(packetID)) {
            this.listeners.set(packetID, []);
        }
        this.listeners.get(packetID)!.push(callback);
    }

    public off(packetID: string, callback: Listener) {
        const packetListeners = this.listeners.get(packetID);
        if (packetListeners) {
            this.listeners.set(packetID, packetListeners.filter(l => l !== callback));
        }
    }

    public send(packetID: string, data: any) {
        if (this.socket && this.socket.readyState === WebSocket.OPEN) {
            const payload = { PacketID: packetID, Data: data };
            this.socket.send(JSON.stringify(payload));
        } else {
            console.warn("[WebSocket] Cannot send message, socket is not open.");
        }
    }

    public close() {
        this.shouldReconnect = false;
        this.socket?.close();
    }
}

export const wsClient = new WebSocketClient("ws://localhost:8080");

function defaultLoadMap(data: any) {
    console.log("[WebSocket] Loading map:", data);
}

function defaultUpdateCars(data: any) {
    console.log("[WebSocket] Updating cars:", data);
}

wsClient.on("map", defaultLoadMap);
wsClient.on("cars", defaultUpdateCars);

export function sendConnectionToken(token: string) {
    wsClient.send("connect", { Token: token });
}

export function useWebSocket(packetID: string, callback: Listener) {
    useEffect(() => {
        wsClient.on(packetID, callback);
        return () => {
            wsClient.off(packetID, callback);
        };
    }, [packetID, callback]);
}
