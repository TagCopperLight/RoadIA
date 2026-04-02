'use client';

import { useState } from 'react';

/**
 * Legend - Légende rétractable affichant informations et raccourcis clavier
 * 
 * **Responsabilités:**
 * 1. Affiche les types de nœuds et leurs couleurs
 * 2. Affiche les types de routes (Standard vs Blocked)
 * 3. Affiche tous les raccourcis clavier disponibles
 * 4. Permet de réduire/agrandir la légende avec animation
 * 
 * **Contenu:**
 * - **Nodes:** Intersection (blue), Workplace (orange), Habitation (green), 
 *   Roundabout (purple), Traffic Light (red)
 * - **Roads:** Standard (gray line), Blocked (red line)
 * - **Shortcuts:** M, V, N, R, DEL, ESC avec leurs fonctions
 * 
 * **Positionnement:** Bas-gauche de la map, z-index: visible au-dessus du canvas
 * 
 * **Animation:** 
 * - Header avec toggle chevron (↓ → ↑ avec rotation 180°)
 * - Contenu slides in/out avec transition 300ms
 * - Hover sur header change couleur de fond
 * 
 * @example
 * // Utilisation dans MapComponent
 * <Legend />
 * // → Affiche en bas-à-gauche de la map
 * // → Cliquable pour réduire/agrandir
 */
export default function Legend() {
	// État local: légende agrandie ou réduite
	const [isExpanded, setIsExpanded] = useState(true);
	return (
		<div className="absolute bottom-[15px] left-[15px] w-[220px] bg-neutral-900 rounded-[12px] shadow-xl text-white overflow-hidden transition-all duration-300">
			{/* ============ HEADER WITH TOGGLE ============ */}
			{/* Click pour agrandir/réduire la légende */}
			<div className="flex items-center justify-between bg-neutral-800 px-4 py-3 cursor-pointer hover:bg-neutral-700 transition-colors" onClick={() => setIsExpanded(!isExpanded)}>
				<p className="text-[13px] font-semibold text-neutral-200">Legend</p>
				{/* Chevron animé: rotate 180° quand isExpanded */}
				<span className={`text-sm transition-transform duration-300 ${isExpanded ? 'rotate-180' : ''}`}>
					▼
				</span>
			</div>

			{/* ============ COLLAPSIBLE CONTENT ============ */}
			{/* S'affiche SEULEMENT si isExpanded === true */}
			{isExpanded && (
				<div className="p-[12px] space-y-[8px]">
					{/* --------- NODE TYPES --------- */}
					<div>
						<p className="text-[11px] text-neutral-400 uppercase tracking-wide mb-[6px]">Nodes</p>
						
						<div className="space-y-[4px]">
							{/* Intersection = Blue */}
							<div className="flex items-center gap-[8px]">
								<div className="w-2 h-2 rounded-full bg-blue-500"></div>
								<span className="text-[11px]">Intersection</span>
							</div>
							{/* Workplace = Orange */}
							<div className="flex items-center gap-[8px]">
								<div className="w-2 h-2 rounded-full bg-orange-500"></div>
								<span className="text-[11px]">Workplace</span>
							</div>
							{/* Habitation = Green */}
							<div className="flex items-center gap-[8px]">
								<div className="w-2 h-2 rounded-full bg-green-500"></div>
								<span className="text-[11px]">Habitation</span>
							</div>
							{/* Roundabout = Purple (not yet implemented in backend) */}
							<div className="flex items-center gap-[8px]">
								<div className="w-2 h-2 rounded-full bg-purple-500"></div>
								<span className="text-[11px]">Roundabout</span>
							</div>
							{/* Traffic Light = Red (not yet implemented in backend) */}
							<div className="flex items-center gap-[8px]">
								<div className="w-2 h-2 rounded-full bg-red-500"></div>
								<span className="text-[11px]">Traffic Light</span>
							</div>
						</div>
					</div>

					{/* --------- ROAD TYPES --------- */}
					<div className="pt-[8px] border-t border-neutral-700">
						<p className="text-[11px] text-neutral-400 uppercase tracking-wide mb-[6px]">Roads</p>
						
						<div className="space-y-[4px]">
							{/* Standard route = Gray line */}
							<div className="flex items-center gap-[8px]">
								<div className="w-4 h-0.5 bg-gray-400"></div>
								<span className="text-[11px]">Standard</span>
							</div>
							{/* Blocked route = Red line */}
							<div className="flex items-center gap-[8px]">
								<div className="w-4 h-0.5 bg-red-500"></div>
								<span className="text-[11px]">Blocked</span>
							</div>
						</div>
					</div>

					{/* --------- KEYBOARD SHORTCUTS --------- */}
					<div className="pt-[8px] border-t border-neutral-700">
						<p className="text-[11px] text-neutral-400 uppercase tracking-wide mb-[6px]">Raccourcis</p>
						
						<div className="space-y-[2px] text-[10px]">
						{/* M = Navigate tool */}
						<div><span className="font-mono bg-neutral-800 px-1.5 py-0.5 rounded">M</span> Naviguer</div>
							{/* V = Select tool */}
						<div><span className="font-mono bg-neutral-800 px-1.5 py-0.5 rounded">V</span> Sélectionner</div>
							{/* N = Add Node tool */}
						<div><span className="font-mono bg-neutral-800 px-1.5 py-0.5 rounded">N</span> Ajouter nœud</div>
							{/* R = Add Road tool */}
						<div><span className="font-mono bg-neutral-800 px-1.5 py-0.5 rounded">R</span> Ajouter route</div>
							{/* DEL = Delete selected */}
						<div><span className="font-mono bg-neutral-800 px-1.5 py-0.5 rounded">DEL</span> Supprimer</div>
							{/* ESC = Deselect */}
						<div><span className="font-mono bg-neutral-800 px-1.5 py-0.5 rounded">ESC</span> Désélectionner</div>
						</div>
					</div>
				</div>
			)}
		</div>
	);
}
