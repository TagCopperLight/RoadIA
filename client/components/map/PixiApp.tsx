import { Application, extend, PixiReactElementProps } from '@pixi/react';
import { Container, Graphics, Sprite, Text } from 'pixi.js';
import { CustomViewport } from './CustomViewport';
import { MapCanvas } from './MapCanvas';
import { MapData, VehicleData } from './types';
import { RefObject, useCallback, useState } from 'react';
import { useMapEditor } from '@/context/MapEditorContext';
import { MAP_CONFIG } from '@/lib/constants';

// Enregistre les types Pixi.js personnalisés avec React Pixi
extend({ Container, Graphics, Sprite, Text, CustomViewport });

// Déclare le type TypeScript pour pixiCustomViewport
declare module "@pixi/react" {
	interface PixiElements {
		pixiCustomViewport: PixiReactElementProps<typeof CustomViewport>;
	}
}

/**
 * @typedef {Object} AppProps
 * @property {RefObject<HTMLElement> | HTMLElement} resizeTo - Conteneur DOM pour redimensionner le canvas
 * @property {MapData | null} mapData - État complet de la map (nœuds et routes)
 * @property {VehicleData[]} vehicles - Liste des véhicules et positions actuelles
 * @property {Function} sendPacket - Fonction pour envoyer des messages WebSocket
 * @property {Function} [onUpdateEdge] - Callback pour modifier une route (optionnel)
 * @property {Function} [onDeleteEdge] - Callback pour supprimer une route (optionnel)
 */
interface AppProps {
	resizeTo: RefObject<HTMLElement> | HTMLElement;
	mapData: MapData | null;
	vehicles: VehicleData[];
	sendPacket: (packetId: string, data: object) => void;
	onUpdateEdge?: (id: number, lane_count: number, speed_limit: number, intersection_type?: string) => void;
	onDeleteEdge?: (id: number) => void;
}

/**
 * PixiApp - Wrapper autour de Pixi.js Application et MapCanvas
 * 
 * **Responsabilités:**
 * 1. Crée l'instance globale Pixi.js Application
 * 2. Initialise le renderer WebGL/Canvas
 * 3. Configure le fond d'écran (background color)
 * 4. Gère l'initialisation du canvas
 * 5. Passe les données à MapCanvas pour la logique d'interaction
 * 
 * **Hiérarchie:**
 * ```
 * MapComponent (reçoit mapData du serveur)
 *  └─ PixiApp (Pixi.js Application)
 *     └─ MapCanvas (Hit-testing, event routing)
 *        └─ scene (nœuds, routes, véhicules)
 * ```
 * 
 * **Initialisation:**
 * 1. Pixi.js crée le renderer
 * 2. `onInit` callback déclenche
 * 3. setIsInitialized(true)
 * 4. MapCanvas peut maintenant accéder `app` via useApplication()
 * 5. MapCanvas rend les nœuds, routes, véhicules
 * 
 * **Props reçuses de MapComponent:**
 * - `resizeTo`: Conteneur HTML à re-renderer si resize
 * - `mapData`: État complet {nodes, edges}
 * - `vehicles`: Positions actuelles
 * - `sendPacket`: Pour communiquer avec le serveur
 * - `onUpdateEdge`, `onDeleteEdge`: Callbacks optionnels
 * 
 * **Props reçuses du Context:**
 * - `activeTool`: Quel outil est sélectionné
 * - `selectedNodeId`: Quel nœud est sélectionné
 * - `isSimulating`: Est-ce en simulation?
 * - `addToast`: Pour afficher des notifications
 * 
 * @example
 * // Utilisation dans MapComponent
 * <PixiApp
 *   resizeTo={container}
 *   mapData={mapData}
 *   vehicles={vehicles}
 *   sendPacket={sendPacket}
 *   onUpdateEdge={...}
 *   onDeleteEdge={...}
 * />
 */
export function PixiApp({ resizeTo, mapData, vehicles, sendPacket, onUpdateEdge, onDeleteEdge }: AppProps) {
	// Récupère l'état global du contexte
	const { activeTool, selectedNodeId, setSelectedNodeId, selectedEdgeId, setSelectedEdgeId, addToast, isSimulating } = useMapEditor();
	
	// État local: le canvas est-il initialisé?
	// Attendu que Pixi.js se monte avant de render MapCanvas
	const [isInitialized, setIsInitialized] = useState(false);
	const handleInit = useCallback(() => setIsInitialized(true), []);

	/**
	 * RENDU: Pixi.js Application + MapCanvas
	 * 
	 * Flux:
	 * 1. <Application> crée le Pixi.js renderer et l'invoque `onInit`
	 * 2. `handleInit` déclenche → setIsInitialized(true)
	 * 3. MapCanvas peut maintenant être réalisé
	 * 4. MapCanvas rend les graphiques (nœuds, routes, véhicules)
	 * 5. MapCanvas configure les event listeners (click, drag, etc.)
	 * 
	 * **Note:** MapCanvas ne monte que si DEUX conditions sont vraies:
	 * - isInitialized === true (Pixi.js est prêt)
	 * - mapData !== null (serveur a envoyé les données)
	 */
	return (
		<Application onInit={handleInit} background={MAP_CONFIG.BACKGROUND_COLOR} resizeTo={resizeTo}>
			{/* MapCanvas ne montage que si l'Application est initialisée ET MapData reçue */}
			{isInitialized && mapData && (
				<MapCanvas
					data={mapData}
					vehicles={vehicles}
					sendPacket={sendPacket}
					onToast={addToast}
					isSimulating={isSimulating}
				/>
			)}
		</Application>
	);
}
