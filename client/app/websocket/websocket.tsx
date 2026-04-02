import { useEffect } from 'react';

/**
 * Type définissant une fonction qui écoute les messages WebSocket
 * @param data - Les données reçues du serveur
 */
type Listener = (data: any) => void;

/**
 * WebSocketClient - Classe de gestion de la connexion WebSocket
 * 
 * Gère la communication bidirectionnelle avec le serveur Rust via WebSocket.
 * Implémente un système de listeners par type de paquet (map, mapEdit, vehicleUpdate, etc.)
 * et une file d'attente pour les messages envoyés avant que la connexion soit établie.
 */
class WebSocketClient {
    private socket: WebSocket | null = null;
    
    /** Map des listeners enregistrés, organisés par type de paquet */
    private listeners: Map<string, Listener[]> = new Map();
    
    private url: string;
    
    /** Intervalle d'attente avant tentative de reconnexion (ms) */
    private reconnectInterval: number = 5000;
    
    /** Drapeau pour contrôler les tentatives de reconnexion automatique */
    private shouldReconnect: boolean = true;

    /** File d'attente des messages à envoyer si la socket n'est pas prête */
    private messageQueue: string[] = [];

    constructor(url: string) {
        this.url = url;
        this.connect();
    }

    /**
     * Établit la connexion WebSocket et configure les événements
     * - Vérifie qu'on est côté client (pas SSR)
     * - Enregistre les handlers pour open, message, close, error
     */
    private connect() {
        // Vérification SSR - ne pas tenter de connexion côté serveur
        if (typeof window === 'undefined') return;

        this.socket = new WebSocket(this.url);

        /**
         * onopen - Appelé quand la connexion est établie
         * Envoie tous les messages en attente dans la queue
         */
        this.socket.onopen = () => {
            console.log(`[WebSocket] Connected to ${this.url}`);
            this.flushQueue();
        };

        /**
         * onmessage - Appelé quand un message arrive du serveur
         * Parse le JSON et dispatch aux listeners correspondants
         * Format: { id: "packetType", data: {...} }
         */
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

        /**
         * onclose - Appelé quand la connexion est fermée
         * Relance automatiquement une tentative de reconnexion après un délai
         */
        this.socket.onclose = () => {
            console.log("[WebSocket] Disconnected");
            if (this.shouldReconnect) {
                setTimeout(() => this.connect(), this.reconnectInterval);
            }
        };

        /**
         * onerror - Appelé en cas d'erreur de connexion
         * Notifie les listeners et ferme la socket
         */
        this.socket.onerror = (event: Event) => {
            const errorMessage = 'WebSocket connection error';
            console.error("[WebSocket] Error:", errorMessage, event);
            
            // Notifie les listeners d'erreur
            this.dispatch('__error__', {
                message: errorMessage,
                timestamp: new Date().toISOString(),
            });
            
            this.socket?.close();
        };
    }

    /**
     * Vide la file d'attente des messages et les envoie au serveur
     * Utilisé après reconnexion pour rattraper les messages perdus
     */
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

    /**
     * Dispatch - Exécute tous les listeners enregistrés pour un type de paquet
     * @param packetID - L'identifiant du type de paquet (ex: "map", "mapEdit")
     * @param data - Les données à passer aux listeners
     */
    private dispatch(packetID: string, data: any) {
        const packetListeners = this.listeners.get(packetID);
        if (packetListeners) {
            // Appelle chaque listener enregistré pour ce type de paquet
            packetListeners.forEach(callback => callback(data));
        } else {
            console.warn(`[WebSocket] No listener for packetID: "${packetID}"`);
        }
    }

    /**
     * Enregistre un listener pour un type de paquet
     * @param packetID - Le type de paquet à écouter
     * @param callback - Fonction appelée quand ce type de paquet arrive
     */
    public on(packetID: string, callback: Listener) {
        if (!this.listeners.has(packetID)) {
            this.listeners.set(packetID, []);
        }
        this.listeners.get(packetID)!.push(callback);
    }

    /**
     * Désenregistre un listener pour un type de paquet
     * @param packetID - Le type de paquet
     * @param callback - La fonction à supprimer
     */
    public off(packetID: string, callback: Listener) {
        const packetListeners = this.listeners.get(packetID);
        if (packetListeners) {
            this.listeners.set(packetID, packetListeners.filter(l => l !== callback));
        }
    }

    /**
     * Envoie un paquet au serveur
     * Si la socket n'est pas prête, le message est mis en queue
     * @param packetID - L'identifiant du type de paquet
     * @param data - Les données à envoyer
     */
    public send(packetID: string, data: any) {
        const payload = JSON.stringify({ id: packetID, data: data });
        if (this.socket && this.socket.readyState === WebSocket.OPEN) {
            this.socket.send(payload);
        } else {
            console.warn("[WebSocket] Socket not open, queueing message");
            // Mise en queue - sera envoyé une fois connecté
            this.messageQueue.push(payload);
        }
    }

    /**
     * Ferme la connexion WebSocket
     * Désactive la reconnexion automatique
     */
    public close() {
        this.shouldReconnect = false;
        this.socket?.close();
    }
}

/** Instance unique du client WebSocket utilisée dans toute l'application */
export const wsClient = new WebSocketClient("ws://localhost:8080/ws");

/**
 * sendConnectionToken - Envoie le token d'authentification au serveur
 * Appelé au démarrage pour établir la session
 * @param token - Token JWT ou autre identifiant de session
 */
export function sendConnectionToken(token: string) {
    wsClient.send("connect", { token: token });
}

/**
 * useWebSocket - Hook React pour écouter les paquets WebSocket
 * 
 * Enregistre un listener et le nettoie automatiquement au unmount
 * @param packetID - Le type de paquet à écouter (ex: "map", "mapEdit", "vehicleUpdate")
 * @param callback - Fonction appelée quand le paquet arrive
 * 
 * @example
 * useWebSocket("map", (data) => {
 *   setMapData(data);
 * });
 */
export function useWebSocket(packetID: string, callback: Listener) {
    useEffect(() => {
        // Enregistre le listener
        wsClient.on(packetID, callback);
        return () => {
            wsClient.off(packetID, callback);
        };
    }, [packetID, callback]);
}
