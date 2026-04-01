import { useEffect } from 'react';

type Listener = (data: unknown) => void;

class WebSocketClient {
    private socket: WebSocket | null = null;
    private listeners: Map<string, Listener[]> = new Map();
    private url: string;
    private reconnectInterval: number = 5000;
    private shouldReconnect: boolean = true;

    private messageQueue: string[] = [];

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
            this.flushQueue();
        };

        this.socket.onmessage = (event: MessageEvent) => {
            try {
                const json = JSON.parse(event.data);
                const packetID = json.id;
                const data = json.data;
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

    private flushQueue() {
        if (this.socket && this.socket.readyState === WebSocket.OPEN) {
            while (this.messageQueue.length > 0) {
                const message = this.messageQueue.shift();
                if (message) {
                    this.socket.send(message);
                }
            }
        }
    }

    private dispatch(packetID: string, data: unknown) {
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

    public send(packetID: string, data: unknown) {
        const payload = JSON.stringify({ id: packetID, data: data });
        if (this.socket && this.socket.readyState === WebSocket.OPEN) {
            this.socket.send(payload);
        } else {
            console.warn("[WebSocket] Socket not open, queueing message");
            this.messageQueue.push(payload);
        }
    }

    public close() {
        this.shouldReconnect = false;
        this.socket?.close();
    }
}

export const wsClient = new WebSocketClient("ws://localhost:8080/ws");

export function sendConnectionToken(token: string) {
    wsClient.send("connect", { token: token });
}

export function useWebSocket(packetID: string, callback: Listener) {
    useEffect(() => {
        wsClient.on(packetID, callback);
        return () => {
            wsClient.off(packetID, callback);
        };
    }, [packetID, callback]);
}
