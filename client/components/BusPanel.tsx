'use client';

import { BusLine, MapData } from './map/types';

interface BusPanelProps {
    busLines: BusLine[];
    mapData: MapData | null;
    pendingStops: number[];
    lineName: string;
    creating: boolean;
    onNameChange: (n: string) => void;
    onRemoveStop: (index: number) => void;
    onCreate: (name: string, stops: number[]) => void;
    onDelete: (id: number) => void;
    onStartCreating: () => void;
    onCancel: () => void;
    onClose: () => void;
}

export default function BusPanel({
    busLines,
    mapData,
    pendingStops,
    lineName,
    creating,
    onNameChange,
    onRemoveStop,
    onCreate,
    onDelete,
    onStartCreating,
    onCancel,
    onClose,
}: BusPanelProps) {
    const nodeMap = mapData
        ? new Map(mapData.nodes.map(n => [n.id, n]))
        : new Map<number, MapData['nodes'][number]>();

    const nodeName = (id: number) => {
        const n = nodeMap.get(id);
        return n ? `${n.kind} #${n.id}` : `#${id}`;
    };

    const canCreate = lineName.trim().length > 0 && pendingStops.length >= 2;

    return (
        <div className="flex-shrink-0 w-64 ml-3 bg-black rounded-[10px] overflow-hidden flex flex-col">
            <div className="flex items-center justify-between px-4 py-3 border-b border-gray-600">
                <span className="text-white font-medium text-sm">Lignes de bus</span>
                <div className="flex items-center gap-2">
                    {creating && (
                        <button
                            onClick={onCancel}
                            className="text-gray-400 hover:text-white transition-colors text-sm leading-none"
                            title="Annuler"
                        >
                            ↩
                        </button>
                    )}
                    <button
                        onClick={onClose}
                        className="text-gray-400 hover:text-white transition-colors text-lg leading-none"
                        title="Fermer"
                    >
                        ×
                    </button>
                </div>
            </div>

            <div className="p-4 overflow-y-auto flex-1 flex flex-col gap-3">
                {creating ? (
                    <>
                        <div className="flex flex-col gap-1">
                            <label className="text-xs text-gray-400 uppercase tracking-wide">Nom de la ligne</label>
                            <input
                                type="text"
                                value={lineName}
                                onChange={e => onNameChange(e.target.value)}
                                placeholder="Ex: Ligne 1"
                                className="bg-gray-800 text-white text-sm rounded px-2 py-1 border border-gray-600 focus:outline-none focus:border-gray-400"
                            />
                        </div>

                        <div className="flex flex-col gap-1">
                            <label className="text-xs text-gray-400 uppercase tracking-wide">
                                Arrêts ({pendingStops.length})
                            </label>
                            {pendingStops.length === 0 ? (
                                <p className="text-xs text-gray-500 italic">
                                    Cliquez sur des nœuds pour ajouter des arrêts.
                                </p>
                            ) : (
                                <div className="flex flex-col gap-1 max-h-48 overflow-y-auto">
                                    {pendingStops.map((nodeId, i) => (
                                        <div key={i} className="flex items-center gap-2 bg-gray-700 rounded px-2 py-1">
                                            <span className="text-white text-xs flex-1">{i + 1}. {nodeName(nodeId)}</span>
                                            <button
                                                onClick={() => onRemoveStop(i)}
                                                className="text-gray-400 hover:text-red-400 transition-colors leading-none"
                                                title="Supprimer"
                                            >
                                                ×
                                            </button>
                                        </div>
                                    ))}
                                </div>
                            )}
                        </div>

                        <button
                            onClick={() => onCreate(lineName, pendingStops)}
                            disabled={!canCreate}
                            className={`mt-1 text-white text-xs rounded px-3 py-1.5 border transition-colors w-full
                                ${canCreate
                                    ? 'bg-blue-900 hover:bg-blue-800 border-blue-700 cursor-pointer'
                                    : 'bg-gray-800 border-gray-700 opacity-40 cursor-not-allowed'}`}
                        >
                            Créer la ligne
                        </button>
                    </>
                ) : (
                    <>
                        {busLines.length === 0 ? (
                            <p className="text-sm text-gray-400">
                                Aucune ligne de bus. Créez-en une pour commencer.
                            </p>
                        ) : (
                            <div className="flex flex-col gap-2">
                                {busLines.map(line => (
                                    <div key={line.id} className="bg-gray-800 rounded px-3 py-2 flex items-start justify-between gap-2">
                                        <div className="flex flex-col gap-0.5 min-w-0">
                                            <span className="text-white text-sm font-medium truncate">{line.name}</span>
                                            <span className="text-gray-400 text-xs">{line.stop_node_ids.length} arrêts</span>
                                        </div>
                                        <button
                                            onClick={() => onDelete(line.id)}
                                            className="text-gray-500 hover:text-red-400 transition-colors leading-none flex-shrink-0 mt-0.5"
                                            title="Supprimer la ligne"
                                        >
                                            ×
                                        </button>
                                    </div>
                                ))}
                            </div>
                        )}

                        <button
                            onClick={onStartCreating}
                            className="mt-auto bg-blue-900 hover:bg-blue-800 text-white text-xs rounded px-3 py-1.5 border border-blue-700 transition-colors w-full"
                        >
                            + Nouvelle ligne
                        </button>
                    </>
                )}
            </div>
        </div>
    );
}
