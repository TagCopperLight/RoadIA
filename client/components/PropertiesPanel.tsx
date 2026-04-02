'use client';

import { useState, useEffect, useCallback } from 'react';
import { useMapEditor } from '@/context/MapEditorContext';
import { MapNode, MapEdge } from './map/types';

/**
 * @typedef {Object} PropertiesPanelProps
 * @property {MapNode[]} nodes - Liste complète des nœuds (pour vérifier les doublons)
 * @property {MapNode | null} selectedNode - Le nœud actuellement sélectionné (null si c'est une route)
 * @property {MapEdge | null} selectedEdge - La route actuellement sélectionnée (null si c'est un nœud)
 * @property {Function} onUpdateNode - Callback pour modifier un nœud
 * @property {Function} onDeleteNode - Callback pour supprimer un nœud
 * @property {Function} onUpdateEdge - Callback pour modifier une route
 * @property {Function} onDeleteEdge - Callback pour supprimer une route
 */
interface PropertiesPanelProps {
	nodes: MapNode[];
	selectedNode: MapNode | null;
	selectedEdge: MapEdge | null;
	onUpdateNode: (id: number, kind: string, name: string) => void;
	onDeleteNode: (id: number) => void;
	onUpdateEdge: (id: number, lane_count: number, speed_limit: number, intersection_type?: string) => void;
	onDeleteEdge: (id: number) => void;
}

/**
 * PropertiesPanel - Formulaire pour éditer les propriétés d'un nœud ou d'une route
 * 
 * **Responsabilités:**
 * 1. Affiche un formulaire pour éditer nœud ou route sélectionné
 * 2. Gère l'état local du formulaire (name, kind, lane_count, etc)
 * 3. Détecte les modifications avec hasNodeChanges / hasEdgeChanges
 * 4. Affiche les boutons "Apply" et "Cancel" SEULEMENT si changements détectés
 * 5. Valide les modifications avant d'appeler les callbacks
 * 6. Bloque l'édition pendant la simulation (isSimulating = true)
 * 
 * **Architecture:**
 * ```
 * MapComponent (reçoit MapData du serveur)
 *  └─ PropertiesPanel (reçoit selectedNode ou selectedEdge)
 *     ├─ État local : nodeName, nodeKind, hasNodeChanges
 *     ├─ État local : laneCount, speedLimit, hasEdgeChanges
 *     └─ Validation : validateNodeName(), handleCommit(), handleCancel()
 *        └─ Envoi : onUpdateNode() ou onUpdateEdge()
 *           └─ Retour à MapComponent
 *              └─ Envoi WebSocket via sendPacket()
 * ```
 * 
 * **Flux de modification d'un nœud:**
 * ```
 * 1. User clique sur un nœud dans PixiApp
 *    ↓
 * 2. MapCanvas appelle setSelectedNodeId()
 *    ↓
 * 3. MapComponent détecte: selectedNode = mapData.nodes.find(id === selectedNodeId)
 *    ↓
 * 4. PropertiesPanel montage:
 *    - useEffect "Sync selectedNode" déclenche
 *    - setNodeName(selectedNode.name)
 *    - setNodeKind(selectedNode.kind)
 *    - setHasNodeChanges(false)  ← cache les boutons Apply/Cancel
 *    ↓
 * 5. User modifie le nom (ex: "node_1" → "Home")
 *    ↓
 * 6. handleChange:
 *    - setNodeName("Home")
 *    - setHasNodeChanges(true)  ← affiche Apply/Cancel
 *    ↓
 * 7. User clique "Apply"
 *    ↓
 * 8. handleNodeCommit():
 *    a. Vérifie isSimulating = false (sinon toast)
 *    b. Valide le nom (pas vide, pas dupliqué)
 *    c. Appelle onUpdateNode(id, "Intersection", "Home")
 *    d. MapComponent.sendPacket('updateNode', {id, kind: "Intersection", name: "Home"})
 *    e. WebSocket envoie au serveur
 *    f. Serveur traite et envoie "mapEdit"
 *    g. MapComponent setMapData()
 *    h. PixiApp re-affiche
 * ```
 * 
 * **Flux de modification d'une route:**
 * Similaire au nœud, mais avec lane_count, speed_limit, intersection_type
 * 
 * @example
 * // Utilisation dans MapComponent
 * {(selectedNode || selectedEdge) && (
 *   <PropertiesPanel
 *     nodes={mapData.nodes}
 *     selectedNode={selectedNode}
 *     selectedEdge={selectedEdge}
 *     onUpdateNode={(id, kind, name) => sendPacket('updateNode', {...})}
 *     onDeleteNode={(id) => sendPacket('deleteNode', {...})}
 *     ...
 *   />
 * )}
 */
export default function PropertiesPanel({
	nodes,
	selectedNode,
	selectedEdge,
	onUpdateNode,
	onDeleteNode,
	onUpdateEdge,
	onDeleteEdge,
}: PropertiesPanelProps) {
	// ============ STATE FROM CONTEXT ============
	// Pour bloca l'édition pendant la simulation
	const { isSimulating, addToast } = useMapEditor();
	
	// ============ NODE FORM STATE ============
	/**
	 * Formulaire pour éditer un nœud sélectionné
	 * 
	 * @state nodeName - Nom du nœud (ex: "node_1", "Home", "Office")
	 * @state nodeKind - Type du nœud (Intersection, Habitation, Workplace, RoundAbout, TrafficLight)
	 * @state nameError - Message d'erreur de validation (si nom invalide)
	 * @state hasNodeChanges - Y a-t-il des modifications non sauvegardées?
	 */
	const [nodeName, setNodeName] = useState('');
	const [nodeKind, setNodeKind] = useState<'Intersection' | 'Habitation' | 'Workplace' | 'RoundAbout' | 'TrafficLight'>('Intersection');
	const [nameError, setNameError] = useState<string | null>(null);
	const [hasNodeChanges, setHasNodeChanges] = useState(false);

	// ============ EDGE FORM STATE ============
	/**
	 * Formulaire pour éditer une route sélectionnée
	 * 
	 * @state laneCount - Nombre de voies (1-6)
	 * @state speedLimit - Limite de vitesse en m/s (1-42)
	 * @state intersectionType - Type d'intersection aux extrémités (Priority, Yield, Stop)
	 * @state hasEdgeChanges - Y a-t-il des modifications non sauvegardées?
	 */
	const [laneCount, setLaneCount] = useState(1);
	const [speedLimit, setSpeedLimit] = useState(40);
	const [intersectionType, setIntersectionType] = useState<'Priority' | 'Yield' | 'Stop'>('Priority');
	const [hasEdgeChanges, setHasEdgeChanges] = useState(false);

	/**
	 * SYNC: Quand le nœud sélectionné change
	 * 
	 * Déclenché quand:
	 * 1. User clique sur un nœud dans la canvas
	 * 2. setSelectedNodeId() est appelé par MapCanvas
	 * 3. MapComponent.selectedNode change
	 * 4. PropertiesPanel reçoit le nouveau selectedNode en props
	 * 5. Ce useEffect déclenche
	 * 
	 * Effet:
	 * - Remplir les champs du formulaire avec les valeurs actuelles
	 * - Cacher les boutons Apply/Cancel (hasNodeChanges = false)
	 */
	useEffect(() => {
		if (selectedNode) {
			setNodeName(selectedNode.name);
			setNodeKind(selectedNode.kind);
			setHasNodeChanges(false);
		}
	}, [selectedNode]);

	/**
	 * SYNC: Quand la route sélectionnée change
	 * 
	 * Déclenché quand:
	 * 1. User clique sur une route dans la canvas
	 * 2. setSelectedEdgeId() est appelé par MapCanvas
	 * 3. MapComponent.selectedEdge change
	 * 4. PropertiesPanel reçoit le nouveau selectedEdge en props
	 * 5. Ce useEffect déclenche
	 * 
	 * Effet:
	 * - Remplir les champs avec les valeurs actuelles
	 * - Cacher les boutons Apply/Cancel
	 */
	useEffect(() => {
		if (selectedEdge) {
			setLaneCount(selectedEdge.lane_count);
			setSpeedLimit(selectedEdge.speed_limit ?? 40);
			setIntersectionType(selectedEdge.intersection_type ?? 'Priority');
			setHasEdgeChanges(false);
		}
	}, [selectedEdge]);

	/**
	 * VALIDATION: Vérifie si le nom du nœud est valide
	 * 
	 * Vérifie:
	 * 1. Nom pas vide
	 * 2. Nom n'existe pas déjà (case-insensitive)
	 * 3. Excepté le nœud actuel (pour permettre "garder le même nom")
	 * 
	 * Retourne:
	 * @returns {string | null} Message d'erreur si invalide, null si valide
	 * 
	 * @example
	 * validateNodeName("") → "Node name cannot be empty"
	 * validateNodeName("Home") → null (valide)
	 * validateNodeName("Office") → "A node with this name already exists" (si Office existe)
	 */
	const validateNodeName = (name: string): string | null => {
		if (!name.trim()) {
			return 'Node name cannot be empty';
		}
		const isDuplicate = nodes.some(n => n.id !== selectedNode?.id && n.name.toLowerCase() === name.toLowerCase());
		if (isDuplicate) {
			return 'A node with this name already exists';
		}
		return null;
	};

	/**
	 * HANDLER: Applique les modifications du nœud
	 * 
	 * Flux:
	 * 1. Vérifie que isSimulating = false (sinon toast d'avertissement)
	 * 2. Valide le nom (pas vide, pas dupliqué)
	 * 3. Appelle onUpdateNode(id, kind, name) → sendPacket() via MapComponent
	 * 4. Cache les boutons Apply/Cancel
	 * 
	 * @example
	 * onClick={() => handleNodeCommit()}
	 * // → onUpdateNode(selectedNode.id, nodeKind, nodeName)
	 * // → MapComponent.sendPacket('updateNode', {id, kind, name})
	 * // → WebSocket au serveur
	 */
	const handleNodeCommit = useCallback(() => {
		if (selectedNode) {
			if (isSimulating) {
				addToast('Arrêtez la simulation pour éditer la carte', 'warning');
				return;
			}
			const error = validateNodeName(nodeName);
			if (error) {
				setNameError(error);
				return;
			}
			setNameError(null);
			onUpdateNode(selectedNode.id, nodeKind, nodeName);
			setHasNodeChanges(false);
		}
	}, [selectedNode, nodeKind, nodeName, onUpdateNode, nodes, isSimulating, addToast]);

	/**
	 * HANDLER: Annule les modifications du nœud
	 * 
	 * Flux:
	 * 1. Restaure les valeurs originales (selectedNode.name, selectedNode.kind)
	 * 2. Efface l'erreur de validation
	 * 3. Cache les boutons Apply/Cancel
	 * 
	 * @example
	 * onClick={() => handleNodeCancel()}
	 * // → Restaure les valeurs d'avant
	 */
	const handleNodeCancel = useCallback(() => {
		if (selectedNode) {
			setNodeName(selectedNode.name);
			setNodeKind(selectedNode.kind);
			setNameError(null);
			setHasNodeChanges(false);
		}
	}, [selectedNode]);

	/**
	 * HANDLER: Applique les modifications de la route
	 * 
	 * Flux:
	 * 1. Vérifie isSimulating = false
	 * 2. Appelle onUpdateEdge(id, laneCount, speedLimit, intersectionType)
	 * 3. Cache les boutons Apply/Cancel
	 * 
	 * @example
	 * onClick={() => handleEdgeCommit()}
	 * // → onUpdateEdge(selectedEdge.id, 3, 50, 'Priority')
	 * // → WebSocket au serveur
	 */
	const handleEdgeCommit = useCallback(() => {
		if (selectedEdge) {
			if (isSimulating) {
				addToast('Arrêtez la simulation pour éditer la carte', 'warning');
				return;
			}
			onUpdateEdge(selectedEdge.id, laneCount, speedLimit, intersectionType);
			setHasEdgeChanges(false);
		}
	}, [selectedEdge, laneCount, speedLimit, intersectionType, onUpdateEdge, isSimulating, addToast]);

	/**
	 * HANDLER: Annule les modifications de la route
	 * 
	 * Flux:
	 * 1. Restaure les valeurs originales
	 * 2. Cache les boutons Apply/Cancel
	 */
	const handleEdgeCancel = useCallback(() => {
		if (selectedEdge) {
			setLaneCount(selectedEdge.lane_count);
			setSpeedLimit(selectedEdge.speed_limit ?? 40);
			setIntersectionType(selectedEdge.intersection_type ?? 'Priority');
			setHasEdgeChanges(false);
		}
	}, [selectedEdge]);

	// ============ STYLES RÉUTILISABLES ============
	// Appliqués à tous les labels et inputs
	const labelClass = 'text-[12px] text-neutral-400 mb-[2px]';
	const inputClass = 'bg-neutral-700 text-white text-[13px] rounded-[6px] px-[8px] py-[4px] w-full outline-none focus:ring-1 focus:ring-yellow-400';

	/**
	 * RENDU: Panneau de propriétés avec deux sections
	 * 
	 * **Section 1: Node Properties** (si selectedNode !== null)
	 * - Affiche les champs Name et Kind
	 * - Boutons Apply/Cancel si hasNodeChanges = true
	 * - Bouton Delete Node
	 * 
	 * **Section 2: Road Properties** (si selectedEdge !== null)
	 * - Affiche les champs Lanes, Speed limit, Type
	 * - Boutons Apply/Cancel si hasEdgeChanges = true
	 * - Bouton Delete Road
	 * 
	 * **Désactivation pendant simulation:**
	 * Si isSimulating = true:
	 * - Tous les inputs sont disabled
	 * - Opacité 50% pour indiquer qu'ils ne sont pas cliquables
	 * - Les outons Apply/Cancel et Delete envoient une toast avertissement
	 * 
	 * **Position:** absolute top-right (avec shadow)
	 * 
	 * @example
	 * // Si une nœud est sélectionné:
	 * <PropertiesPanel selectedNode={{id: 1, name: "Home", kind: "Habitation"}} />
	 * 
	 * // → Affiche 2 champs Name et Kind
	 * // → User modifie → boutons Apply/Cancel apparaissent
	 * // → Click Apply → onUpdateNode() → sendPacket('updateNode')
	 */
	return (
		<div className="absolute top-[15px] right-[15px] w-[260px] bg-neutral-900 rounded-[12px] p-[14px] shadow-xl text-white flex flex-col gap-[10px]">
			{/* ============= NODE PROPERTIES ============= */}
			{selectedNode && (
				<>
					<p className="text-[14px] font-semibold text-neutral-200">Node Properties</p>

					{/* Name input */}
					<div>
						<p className={labelClass}>Name</p>
						<input
							// Désactivé pendant simulation
							disabled={isSimulating}
							className={`${inputClass} ${nameError ? 'ring-1 ring-red-500' : ''} ${isSimulating ? 'opacity-50 cursor-not-allowed' : ''}`}
							value={nodeName}
							onChange={e => {
								setNodeName(e.target.value);
								setHasNodeChanges(true);  // Affiche Apply/Cancel
								// Valide au fur et à mesure (feedback en temps réel)
								const error = validateNodeName(e.target.value);
								setNameError(error);
							}}
						/>
						{/* Affiche l'erreur sous le champ */}
						{nameError && <p className="text-[11px] text-red-400 mt-[4px]">{nameError}</p>}
					</div>

					{/* Kind select */}
					<div>
						<p className={labelClass}>Kind</p>
						<select
							disabled={isSimulating}
							className={`${inputClass} ${isSimulating ? 'opacity-50 cursor-not-allowed' : ''}`}
							value={nodeKind}
							onChange={e => {
								setNodeKind(e.target.value as typeof nodeKind);
								setHasNodeChanges(true);  // Affiche Apply/Cancel
							}}
						>
							<option value="Intersection">Intersection</option>
							<option value="Habitation">Habitation</option>
							<option value="Workplace">Workplace</option>
							<option value="RoundAbout">RoundAbout</option>
							<option value="TrafficLight">TrafficLight</option>
						</select>
					</div>

					{/* Boutons Apply/Cancel (affichés SEULEMENT si hasNodeChanges = true) */}
					{hasNodeChanges && (
						<div className="flex gap-[8px]">
							<button
								className="flex-1 bg-green-600 hover:bg-green-500 text-white text-[13px] font-medium py-[5px] rounded-[6px] cursor-pointer transition-colors"
								onClick={handleNodeCommit}
							>
								Appliquer
							</button>
							<button
								className="flex-1 bg-neutral-600 hover:bg-neutral-500 text-white text-[13px] font-medium py-[5px] rounded-[6px] cursor-pointer transition-colors"
								onClick={handleNodeCancel}
							>
								Annuler
							</button>
						</div>
					)}

					{/* Bouton Delete Node */}
					<button
						disabled={isSimulating}
						className={`mt-[4px] bg-red-600 hover:bg-red-500 text-white text-[13px] font-medium py-[5px] rounded-[6px] cursor-pointer transition-colors ${isSimulating ? 'opacity-50 cursor-not-allowed' : ''}`}
						onClick={() => onDeleteNode(selectedNode.id)}
					>
						Supprimer le nœud
					</button>
				</>
			)}

			{/* ============= ROAD PROPERTIES ============= */}
			{selectedEdge && (
				<>
					<p className="text-[14px] font-semibold text-neutral-200">Road Properties</p>

					{/* Lanes input */}
					<div>
						<p className={labelClass}>Lanes</p>
						<input
							disabled={isSimulating}
							type="number"
							min={1}
							max={6}
							className={`${inputClass} ${isSimulating ? 'opacity-50 cursor-not-allowed' : ''}`}
							value={laneCount}
							onChange={e => {
								setLaneCount(Number(e.target.value));
								setHasEdgeChanges(true);  // Affiche Apply/Cancel
							}}
						/>
					</div>

					{/* Speed limit input */}
					<div>
						<p className={labelClass}>Speed limit (m/s)</p>
						<input
							disabled={isSimulating}
							type="number"
							min={1}
							max={42}
							className={`${inputClass} ${isSimulating ? 'opacity-50 cursor-not-allowed' : ''}`}
							value={speedLimit}
							onChange={e => {
								setSpeedLimit(Number(e.target.value));
								setHasEdgeChanges(true);  // Affiche Apply/Cancel
							}}
						/>
					</div>

					{/* Intersection type select */}
					<div>
						<p className={labelClass}>Type</p>
						<select
							disabled={isSimulating}
							className={`${inputClass} ${isSimulating ? 'opacity-50 cursor-not-allowed' : ''}`}
							value={intersectionType}
							onChange={e => {
								setIntersectionType(e.target.value as 'Priority' | 'Yield' | 'Stop');
								setHasEdgeChanges(true);  // Affiche Apply/Cancel
							}}
						>
							<option value="Priority">Priority</option>
							<option value="Yield">Yield</option>
							<option value="Stop">Stop</option>
						</select>
					</div>

					{/* Boutons Apply/Cancel (affichés SEULEMENT si hasEdgeChanges = true) */}
					{hasEdgeChanges && (
						<div className="flex gap-[8px]">
							<button
								className="flex-1 bg-green-600 hover:bg-green-500 text-white text-[13px] font-medium py-[5px] rounded-[6px] cursor-pointer transition-colors"
								onClick={handleEdgeCommit}
							>
								Appliquer
							</button>
							<button
								className="flex-1 bg-neutral-600 hover:bg-neutral-500 text-white text-[13px] font-medium py-[5px] rounded-[6px] cursor-pointer transition-colors"
								onClick={handleEdgeCancel}
							>
								Annuler
							</button>
						</div>
					)}

					{/* Bouton Delete Road */}
					<button
						disabled={isSimulating}
						className={`mt-[4px] bg-red-600 hover:bg-red-500 text-white text-[13px] font-medium py-[5px] rounded-[6px] cursor-pointer transition-colors ${isSimulating ? 'opacity-50 cursor-not-allowed' : ''}`}
						onClick={() => onDeleteEdge(selectedEdge.id)}
					>
						Supprimer la route
					</button>
				</>
			)}
		</div>
	);
}
