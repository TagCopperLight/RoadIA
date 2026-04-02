'use client';

import Image from 'next/image';
import { useEffect, useState } from 'react';
import { wsClient } from '@/app/websocket/websocket';
import { useMapEditor } from '@/context/MapEditorContext';

/**
 * @typedef {Object} EditTool
 * @property {string} icon - Fichier SVG de l'icône (sans extension)
 * @property {string} alt - Texte alternatif pour le hover tooltip
 * @property {string} tool - ID interne de l'outil ('navigate', 'select', 'addNode', 'addRoad')
 */
const EDIT_TOOLS: { icon: string; alt: string; tool: string }[] = [
    { icon: 'Hand', alt: 'Naviguer', tool: 'navigate' },
    { icon: 'Move', alt: 'Sélectionner', tool: 'select' },
    { icon: 'House', alt: 'Ajouter nœud', tool: 'addNode' },
    { icon: 'Edit', alt: 'Ajouter route', tool: 'addRoad' },
];

/**
 * Toolbar - Barre d'outils pour sélectionner les outils et contrôler la simulation
 * 
 * **Responsabilités:**
 * 1. Affiche 4 outils d'édition : Navigate, Select, AddNode, AddRoad
 * 2. Gère la sélection de l'outil actif (highlight l'outil choisi)
 * 3. Affiche les contrôles de simulation: Play/Pause et Reset
 * 4. Gère les raccourcis clavier : M, V, N, R pour changer d'outil
 * 
 * **Outils d'édition:**
 * - **Navigate** (M) : Zoome/défile la map
 * - **Select** (V) : Clique sur nœuds/routes pour les sélectionner
 * - **AddNode** (N) : Clique sur espace vide pour créer un nœud
 * - **AddRoad** (R) : Clique 2 nœuds pour créer une route
 * 
 * **Simulation:**
 * - **Play** : startSimulation() → véhicules se déplacent
 * - **Pause** : stopSimulation() → véhicules s'arrêtent
 * - **Reset** : resetSimulation() → réinitialise positions
 */
export default function Toolbar() {
    // Récupère et modifie l'état global du contexte
    const { activeTool, setActiveTool, isSimulating, setIsSimulating } = useMapEditor();
    
    // État local pour l'animation Play/Pause
    const [isPlaying, setIsPlaying] = useState(false);

    /**
     * HANDLER: Alterne entre Play et Pause
     * 
     * **Quand on clique sur Play:**
     * 1. Envoie 'startSimulation' au serveur
     * 2. Le serveur commence la boucle de simulation (physics engine)
     * 3. Véhicules commencent à se déplacer
     * 4. Serveur envoie 'vehicleUpdate' chaque frame
     * 5. MapComponent.setVehicles() re-affiche
     * 
     * **État UI:**
     * - setActiveTool('navigate') : Force Navigate mode (pas d'édition)
     * - setIsSimulating(true) : PropertiesPanel et MapCanvas sont grisés
     * - setIsPlaying(true) : Bouton affiche l'icône Pause
     * 
     * **Quand on clique sur Pause:**
     * 1. Envoie 'stopSimulation' au serveur
     * 2. Le serveur arrête la boucle de simulation
     * 3. Véhicules s'arrêtent (vehicleUpdate s'arrête)
     * 4. setIsSimulating(false) : Édition réactivée
     */
    const handlePlayPause = () => {
        if (isPlaying) {
            // PAUSE mode
            wsClient.send('stopSimulation', {});
            setIsPlaying(false);
            setIsSimulating(false);
        } else {
            // PLAY mode
            wsClient.send('startSimulation', {});
            setActiveTool('navigate');  // Force navigate pour éviter édition
            setIsPlaying(true);
            setIsSimulating(true);
        }
    };

    /**
     * HANDLER: Réinitialise la simulation
     * 
     * Envoie 'resetSimulation' au serveur qui:
     * 1. Remet tous les véhicules à leur position initiale (origin_id)
     * 2. Arrête la simulation (stopSimulation interne)
     * 3. Envoie 'vehicleUpdate' avec les positions initiales
     * 4. MapComponent.setVehicles() affiche les véhicules reset
     */
    const handleReset = () => {
        wsClient.send('resetSimulation', {});
        setIsPlaying(false);
        setIsSimulating(false);
    };

    /**
     * KEYBOARD SHORTCUTS for tool selection
     * 
     * Déclenché quand l'utilisateur appuie sur une touche (si pas dans un input)
     * 
     * Raccourcis:
     * - M/m : Navigate (zoom + défilement)
     * - V/v : Select (sélectionner nœuds/routes)
     * - N/n : Add Node (créer nœud)
     * - R/r : Add Road (créer route)
     * 
     * **Détection d'input:** Si utilisateur tape dans un champ (input/select),
     * les raccourcis sont IGNORÉS pour éviter les conflits de saisie.
     */
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            // Ignore si utilisateur tape dans un champ
            if (e.target instanceof HTMLInputElement || e.target instanceof HTMLSelectElement) return;
            switch (e.key) {
                case 'm': case 'M':
                    setActiveTool('navigate');
                    break;
                case 'v': case 'V':
                    setActiveTool('select');
                    break;
                case 'n': case 'N':
                    setActiveTool('addNode');
                    break;
                case 'r': case 'R':
                    setActiveTool('addRoad');
                    break;
            }
        };
        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [setActiveTool]);

    /**
     * RENDU: Barre d'outils avec sections Edit et Simulation
     * 
     * **Structure:**
     * - Edit Tools (Navigate, Select, AddNode, AddRoad) avec séparateurs
     * - Séparateur principal
     * - Simulation Tools (Play/Pause, Reset)
     * 
     * **Styles:**
     * - Opacité 100% si activeTool correspond, 50% sinon
     * - Hover opacity 75% pour feedback visuel
     */
    return (
        <div className="flex items-center w-full pl-[15px] pr-[15px]">
            <div className='flex items-center bg-black rounded-[10px] w-full'>
                {/* ============ EDIT TOOLS SECTION ============ */}
                {/* Outils d'édition : Navigate, Select, AddNode, AddRoad */}
                {EDIT_TOOLS.map((tool, index) => (
                    <div key={tool.tool} className="flex items-center">
                        <div
                            onClick={() => setActiveTool(tool.tool as any)}
                            className={`flex items-center cursor-pointer transition-opacity ${activeTool === tool.tool ? 'opacity-100' : 'opacity-50 hover:opacity-75'}`}
                            title={tool.alt}
                        >
                            <Image src={`/map/${tool.icon}.svg`} alt={tool.alt} width={24} height={24} className='m-[11px]' />
                        </div>
                        {index < EDIT_TOOLS.length - 1 && (
                            <Image src="/map/Separator.svg" alt="Séparateur" height={26} width={1} />
                        )}
                    </div>
                ))}

                {/* Separator between Edit and Simulation tools */}
                <Image src="/map/Separator.svg" alt="Séparateur" height={26} width={1} className='mx-[4px]' />

                {/* ============ SIMULATION TOOLS SECTION ============ */}
                {/* Contrôles Play/Pause et Reset pour la simulation */}
                <div onClick={handlePlayPause} className="flex items-center cursor-pointer hover:opacity-50 transition-opacity" title={isPlaying ? 'Pause' : 'Jouer'}>
                    <Image src={isPlaying ? '/map/Pause.svg' : '/map/Play.svg'} alt={isPlaying ? 'Pause' : 'Jouer'} width={24} height={24} className='m-[11px]' />
                </div>
                <Image src="/map/Separator.svg" alt="Séparateur" height={26} width={1} />
                <div onClick={handleReset} className="flex items-center cursor-pointer hover:opacity-50 transition-opacity" title="Réinitialiser">
                    <Image src="/map/Reset.svg" alt="Réinitialiser" width={24} height={24} className='m-[11px]' />
                </div>
            </div>
        </div>
    );
}
