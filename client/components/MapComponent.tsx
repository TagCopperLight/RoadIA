'use client';

import Image from "next/image";
import { useCallback, useState, useEffect, useRef } from 'react';
import { sendConnectionToken, useWebSocket, wsClient } from '@/app/websocket/websocket';
import { PixiApp } from './map/PixiApp';
import { MapData, VehicleData } from './map/types';
import PropertiesPanel from './PropertiesPanel';
import Legend from './Legend';
import { useMapEditor } from '@/context/MapEditorContext';

/**
 * @typedef {Object} MapComponentProps
 * @property {string} uuid - Unique identifier for the map
 */
interface MapComponentProps {
	uuid: string;
}

/**
 * MapComponent - Main container for the interactive map
 * 
 * **Responsabilités:**
 * 1. Établit la connexion WebSocket au serveur via sendConnectionToken()
 * 2. Écoute les 3 paquets serveur principaux:
 *    - "map" : État initial + reconnexion
 *    - "mapEdit" : Mises à jour après éditions (avec auto-création de véhicules)
 *    - "vehicleUpdate" : Positions des véhicules chaque frame
 * 3. Passe les données à PixiApp pour le rendu
 * 4. Gère les actions du utilisateur (sendPacket)
 * 5. Affiche PropertiesPanel si nœud/route sélectionné
 * 6. Affiche Legend (légende rétractable)
 * 
 * **Architecture des données:**
 * ```
 * WebSocket              MapComponent          PixiApp (Render)       PropertiesPanel (Edit)
 * "map"       ---------> setMapData() ------>  "Display nodes/roads"
 * "mapEdit"   ---------> setMapData() ------>  "Re-render"
 * "vehicleUpdate" -----> setVehicles() -----> "Animate vehicles"
 *
 * User click on node  ---> PixiApp emit --> setSelectedNodeId() --> PropertiesPanel show
 * User click "Apply" ---> PropertiesPanel --> sendPacket() --> WebSocket --> Server
 * ```
 * 
 * **Auto-création de véhicules:**
 * Quand un nœud Habitation ou Workplace est créé:
 * 1. useWebSocket("mapEdit") détecte le nouveau nœud
 * 2. sendPacket('createVehicle', { origin_id })
 * 3. Serveur crée un véhicule, envoie vehicleUpdate
 * 4. PixiApp affiche le véhicule en mouvement
 * 
 * @example
 * // Utilisation dans app/map/[uuid]/page.tsx
 * <MapComponent uuid={uuid} />
 */
export default function MapComponent({ uuid }: MapComponentProps) {
	// ============ STATE FROM CONTEXT ============
	// Partagé avec tous les autres composants via MapEditorProvider
	const { selectedNodeId, setSelectedNodeId, selectedEdgeId, setSelectedEdgeId, addToast, isSimulating } = useMapEditor();
	
	// ============ LOCAL STATE ============
	// Référence au conteneur DOM pour Pixi.js
	const [container, setContainer] = useState<HTMLDivElement | null>(null);
	
	// État actuel de la map (nœuds + routes)
	const [mapData, setMapData] = useState<MapData | null>(null);
	
	// Liste des véhicules et leurs positions (mise à jour chaque frame)
	const [vehicles, setVehicles] = useState<VehicleData[]>([]);
	
	// Cache des véhicules précédents pour calculer heading et speed
	const prevVehiclesRef = useRef<Record<number, VehicleData>>({});

	/**
	 * Enregistre le conteneur DOM dans le state
	 * Appelé via <div ref={onRefChange}> pour que Pixi.js puisse y dessiner
	 * 
	 * @param {HTMLDivElement} node - Le div conteneur
	 */
	const onRefChange = useCallback((node: HTMLDivElement) => {
		setContainer(node);
	}, []);

	/**
	 * Wrapper autour de wsClient.send() pour envoyer des paquets au serveur
	 * Utilisé par:
	 * - PixiApp quand user crée/modifie/supprime nœud/route
	 * - MapComponent auto-création de véhicules
	 * 
	 * @param {string} packetId - Type de paquet ('addNode', 'updateNode', 'createVehicle', etc)
	 * @param {object} data - Données du paquet
	 * 
	 * @example
	 * sendPacket('addNode', { x: 100, y: 200, kind: 'Intersection', name: 'node_1' })
	 * sendPacket('updateRoad', { id: 5, lane_count: 3, speed_limit: 50 })
	 */
	const sendPacket = useCallback((packetId: string, data: object) => {
		wsClient.send(packetId, data);
	}, []);

	/**
	 * Au montage: établit la connexion WebSocket
	 * Envoie le token d'auth au serveur (dummy token: "auth-token")
	 * Le serveur répond avec "map" paquet contenant l'état complet
	 */
	useEffect(() => {
		sendConnectionToken("auth-token");
	}, []);

	/**
	 * LISTENER: Reçoit l'état COMPLET de la map
	 * 
	 * Déclenché:
	 * 1. Au démarrage (après sendConnectionToken)
	 * 2. À reconnexion après déconnexion
	 * 3. À chargement d'une nouvelle map
	 * 
	 * @param {MapData} data - { nodes: Node[], edges: Edge[] }
	 */
	useWebSocket("map", (data) => {
		setMapData(data as MapData);
	});

	/**
	 * LISTENER: Reçoit les modifications de la map (après qu'un utilisateur édite)
	 * 
	 * Flux:
	 * 1. User crée nœud Habitation via PixiApp
	 * 2. sendPacket('addNode', {...}) → WebSocket
	 * 3. Serveur traite, crée le nœud
	 * 4. Serveur diffuse "mapEdit" à TOUS les clients
	 * 5. useWebSocket("mapEdit") reçoit la nouvelle map
	 * 6. setMapData() déclenche l'update PixiApp
	 * 7. NEW: Si nœud est Habitation/Workplace → auto-crée un véhicule
	 * 
	 * **Auto-création de véhicules:**
	 * Après chaque addNode, on compare:
	 * - data.nodes (nouveaux nœuds du serveur)
	 * - mapData.nodes (ancien state)
	 * 
	 * Pour chaque nœud NOUVEAU qui est Habitation/Workplace:
	 * → sendPacket('createVehicle', { origin_id: node.id })
	 * → Serveur crée le véhicule, envoie vehicleUpdate
	 * 
	 * @param {object} data - Response du serveur
	 * @param {boolean} data.success - Opération réussie?
	 * @param {Node[]} data.nodes - Nouvelle liste des nœuds
	 * @param {Edge[]} data.edges - Nouvelle liste des routes
	 * @param {string} [data.error] - Message d'erreur si success=false
	 * 
	 * @example
	 * // User crée Habitation "Home1"
	 * // → Server envoie:
	 * { success: true, nodes: [..., {id: 3, name: 'Home1', kind: 'Habitation', x: 100, y: 150}], edges: [...] }
	 * // → MapComponent détecte id 3 n'existe pas dans mapData.nodes && kind='Habitation'
	 * // → sendPacket('createVehicle', { origin_id: 3 })
	 */
	useWebSocket("mapEdit", useCallback((data: any) => {
		if (data.success) {
			const newMapData = { nodes: data.nodes, edges: data.edges };
			
			// Détecte les NOUVEAUX nœuds de type Habitation ou Workplace
			if (mapData) {
				const newNodes = data.nodes.filter((n: any) => 
					!mapData.nodes.find(old => old.id === n.id) &&
					(n.kind === 'Habitation' || n.kind === 'Workplace')
				);
				
				// Auto-crée un véhicule pour chaque nouveau nœud Habitation/Workplace
				// Le serveur répond avec "vehicleUpdate" paquet
				newNodes.forEach((node: any) => {
					sendPacket('createVehicle', { origin_id: node.id });
				});
			}
			
			// Met à jour le state pour que Pixi re-affiche
			setMapData(newMapData);
		} else {
			console.error("[MapEdit] Error:", data.error);
		}
	}, [mapData, sendPacket]));

	/**
	 * LISTENER: Reçoit les positions actuelles de TOUS les véhicules
	 * 
	 * Déclenché:
	 * - Pendant la simulation: chaque frame (~60Hz)
	 * - Contient les coordonnées x, y de chaque véhicule
	 * 
	 * Calcul du heading (direction l'orientation du véhicule):
	 * 1. Compare position actuelle vs position précédente
	 * 2. Calcule l'angle: atan2(dy, dx)
	 * 3. Calcule la vitesse : sqrt(dx² + dy²)
	 * 4. Stocke dans prevVehiclesRef pour la frame suivante
	 * 
	 * **Pourquoi ce calcul?**
	 * Le serveur envoie JUSTE la position (x, y).
	 * Mais PixiApp a besoin de heading pour afficher le véhicule dans la bonne direction!
	 * Donc on calcule l'angle de rotation basé sur le MOUVEMENT.
	 * 
	 * @param {object} data - Paquet du serveur
	 * @param {VehicleData[]} data.vehicles - Positions actuelles des véhicules
	 * 
	 * @example
	 * // Serveur envoie:
	 * { vehicles: [{id: 1, x: 100, y: 150}, {id: 2, x: 200, y: 250}] }
	 * 
	 * // MapComponent calcule:
	 * Vehicle 1: heading = atan2(150 - 150_prev, 100 - 100_prev) = atan2(differenceY, differenceX)
	 * Vehicle 1: speed = sqrt((dx)^2 + (dy)^2)
	 */
	useWebSocket("vehicleUpdate", (data: any) => {
        if (data && Array.isArray(data.vehicles)) {
			const newVehicles = data.vehicles as VehicleData[];
			const processedVehicles = newVehicles.map(vehicle => {
				const prevVehicle = prevVehiclesRef.current[vehicle.id];
				if (prevVehicle) {
					const dx = vehicle.x - prevVehicle.x;
					const dy = vehicle.y - prevVehicle.y;
					const dist = Math.sqrt(dx * dx + dy * dy);
					if (dist > 0.01) {
						vehicle.heading = Math.atan2(dy, dx);
						vehicle.speed = dist;
					} else {
						vehicle.heading = prevVehicle.heading;
						vehicle.speed = 0;
					}
				} else {
                    vehicle.heading = undefined;
                    vehicle.speed = 0;
                }
				return vehicle;
			});

			const newPrevVehicles: Record<number, VehicleData> = {};
			processedVehicles.forEach(v => {
				newPrevVehicles[v.id] = v;
			});
			prevVehiclesRef.current = newPrevVehicles;
		    setVehicles(processedVehicles);
        }
	});

	// ============ SÉLECTION COURANTE ============
	// Récupère le nœud/route sélectionné avec les IDs du context
	const selectedNode = mapData?.nodes.find(n => n.id === selectedNodeId) ?? null;
	const selectedEdge = mapData?.edges.find(e => e.id === selectedEdgeId) ?? null;

	/**
	 * Rendu du composant:
	 * 1. <div ref={onRefChange}> = Conteneur Pixi.js
	 * 2. <PixiApp> = Canvas interactif (rendu + hit-testing)
	 * 3. <PropertiesPanel> = Formulaire d'édition (si nœud/route sélectionné)
	 * 4. Orange man = Logo interactif
	 * 5. <Legend> = Légende rétractable bas-gauche
	 * 
	 * **Hiérarchie:**
	 * MapComponent (Main)
	 *  ├─ PixiApp (Rendering)
	 *  │  └─ MapCanvas (Hit-testing + Events)
	 *  ├─ PropertiesPanel (Editing - conditionnel)
	 *  └─ Legend (Info)
	 */
	return (
		<div ref={onRefChange} className="w-full h-full rounded-[10px] overflow-hidden relative">
			{/* Pixi.js Canvas pour afficher nœuds, routes, véhicules */}
			{container && (
				<PixiApp
					resizeTo={container}
					mapData={mapData}
					vehicles={vehicles}
					sendPacket={sendPacket}
					// Quand user modifie une route via la properties panel
					onUpdateEdge={(id: number, lane_count: number, speed_limit: number, intersection_type?: string) =>
						sendPacket('updateRoad', { id, lane_count, speed_limit, intersection_type })
					}
					// Quand user supprime une route via la properties panel
					onDeleteEdge={(id: number) => {
						sendPacket('deleteRoad', { id });
						setSelectedEdgeId(null);
					}}
				/>
			)}

			{/* Formulaire d'édition des propriétés (nœud ou route) */}
			{/* S'affiche SEULEMENT si nœud OU route sélectionné */}
			{(selectedNode || selectedEdge) && (
				<PropertiesPanel
					nodes={mapData?.nodes ?? []}
					selectedNode={selectedNode}
					selectedEdge={selectedEdge}
					// Envoie la modification au serveur
					onUpdateNode={(id: number, kind: string, name: string) =>
						sendPacket('updateNode', { id, kind, name })
					}
					// Supprime le nœud et déselectionne
					onDeleteNode={(id: number) => {
						sendPacket('deleteNode', { id });
						setSelectedNodeId(null);
					}}
					// Modifie la route
					onUpdateEdge={(id: number, lane_count: number, speed_limit: number, intersection_type?: string) =>
						sendPacket('updateRoad', { id, lane_count, speed_limit, intersection_type })
					}
					// Supprime la route et déselectionne
					onDeleteEdge={(id: number) => {
						sendPacket('deleteRoad', { id });
						setSelectedEdgeId(null);
					}}
				/>
			)}

			{/* Logo "Orange man" en bas-à-droite */}
			<div className="absolute bottom-[15px] right-[15px] bg-white p-1 rounded-[10px] shadow-md group cursor-pointer">
				<Image src="/map/man.png" alt="Mascotte" width={35} height={35} className="transition-transform duration-200 group-hover:-rotate-12" />
			</div>

			{/* Légende rétractable (Node types, Road types, Shortcuts) */}
			<Legend />
		</div>
	);
}
