'use client';

import { VehicleSummary, MapData, Motorization } from './map/types';

const MOTOR_COLORS: Record<Motorization, string> = {
    Hybride:    'text-purple-400',
    Electrique: 'text-cyan-400',
    Essence:    'text-amber-400',
    Diesel:     'text-yellow-700',
};

const MOTOR_LABELS: Record<Motorization, string> = {
    Hybride:    'Hybride',
    Electrique: 'Électrique',
    Essence:    'Essence',
    Diesel:     'Diesel',
};

interface WaypointPanelProps {
    vehicleSummaries: VehicleSummary[];
    mapData: MapData | null;
    waypointNodeId: number | null;
    waypointVehicleId: number | null;
    pendingWaypoints: number[];
    onSelectVehicle: (id: number) => void;
    onRemoveWaypoint: (index: number) => void;
    onApply: () => void;
    onCancel: () => void;
    onClose: () => void;
}

export default function WaypointPanel({
    vehicleSummaries,
    mapData,
    waypointNodeId,
    waypointVehicleId,
    pendingWaypoints,
    onSelectVehicle,
    onRemoveWaypoint,
    onApply,
    onCancel,
    onClose,
}: WaypointPanelProps) {
    const nodeMap = mapData
        ? new Map(mapData.nodes.map(n => [n.id, n]))
        : new Map<number, MapData['nodes'][number]>();

    const nodeName = (id: number) => {
        const n = nodeMap.get(id);
        return n ? `${n.kind} #${n.id}` : `#${id}`;
    };

    let content: React.ReactNode;
    const showCancel = waypointVehicleId !== null;

    // State 5: Vehicle detail
    if (waypointVehicleId !== null) {
        const summary = vehicleSummaries.find(v => v.id === waypointVehicleId);
        const motorization = summary?.motorization as Motorization | undefined;
        content = (
            <div className="flex flex-col gap-3">
                <div className="flex flex-col gap-1">
                    <label className="text-xs text-gray-400 uppercase tracking-wide">Véhicule</label>
                    <span className="text-white text-sm">#{waypointVehicleId}</span>
                </div>

                {motorization && (
                    <div className="flex flex-col gap-1">
                        <label className="text-xs text-gray-400 uppercase tracking-wide">Type</label>
                        <span className={`text-sm ${MOTOR_COLORS[motorization]}`}>{MOTOR_LABELS[motorization]}</span>
                    </div>
                )}

                {summary && (
                    <>
                        <div className="flex flex-col gap-1">
                            <label className="text-xs text-gray-400 uppercase tracking-wide">Origine</label>
                            <span className="text-white text-sm">{nodeName(summary.origin_id)}</span>
                        </div>
                        <div className="flex flex-col gap-1">
                            <label className="text-xs text-gray-400 uppercase tracking-wide">Destination</label>
                            <span className="text-white text-sm">{nodeName(summary.destination_id)}</span>
                        </div>
                    </>
                )}

                <div className="flex flex-col gap-1">
                    <label className="text-xs text-gray-400 uppercase tracking-wide">
                        Waypoints en attente ({pendingWaypoints.length})
                    </label>
                    {pendingWaypoints.length === 0 ? (
                        <p className="text-xs text-gray-500 italic">
                            Cliquez sur des nœuds pour ajouter des waypoints.
                        </p>
                    ) : (
                        <div className="flex flex-col gap-1 max-h-48 overflow-y-auto">
                            {pendingWaypoints.map((nodeId, i) => (
                                <div key={i} className="flex items-center gap-2 bg-gray-700 rounded px-2 py-1">
                                    <span className="text-white text-xs flex-1">{i + 1}. {nodeName(nodeId)}</span>
                                    <button
                                        onClick={() => onRemoveWaypoint(i)}
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
                    onClick={onApply}
                    className="mt-1 bg-blue-900 hover:bg-blue-800 text-white text-xs rounded px-3 py-1.5 border border-blue-700 transition-colors w-full"
                >
                    Appliquer
                </button>
            </div>
        );
    }

    else if (waypointNodeId !== null) {
        const clickedNode = nodeMap.get(waypointNodeId);
        const isIntersection = clickedNode?.kind === 'Intersection';

        if (isIntersection) {
            content = (
                <div className="flex flex-col gap-3">
                    <div className="flex flex-col gap-1">
                        <label className="text-xs text-gray-400 uppercase tracking-wide">Nœud</label>
                        <span className="text-white text-sm">Intersection #{waypointNodeId}</span>
                    </div>
                    <p className="text-sm text-gray-500">
                        Cliquez sur un <span className="text-blue-400">Habitation</span> ou un <span className="text-red-400">Travail</span> pour voir les véhicules associés.
                    </p>
                </div>
            );
        } else {
            const matchingVehicles = vehicleSummaries.filter(
                v => v.origin_id === waypointNodeId || v.destination_id === waypointNodeId
            );

            if (matchingVehicles.length === 0) {
                content = (
                    <div className="flex flex-col gap-3">
                        <div className="flex flex-col gap-1">
                            <label className="text-xs text-gray-400 uppercase tracking-wide">Nœud</label>
                            <span className="text-white text-sm">
                                {clickedNode ? `${clickedNode.kind} #${clickedNode.id}` : `#${waypointNodeId}`}
                            </span>
                        </div>
                        <p className="text-sm text-gray-500 italic">
                            Aucun véhicule ne passe par ce nœud.
                        </p>
                    </div>
                );
            } else {
                content = (
                    <div className="flex flex-col gap-3">
                        <div className="flex flex-col gap-1">
                            <label className="text-xs text-gray-400 uppercase tracking-wide">Nœud</label>
                            <span className="text-white text-sm">
                                {clickedNode ? `${clickedNode.kind} #${clickedNode.id}` : `#${waypointNodeId}`}
                            </span>
                        </div>
                        <div className="flex flex-col gap-1">
                            <label className="text-xs text-gray-400 uppercase tracking-wide">
                                Véhicules ({matchingVehicles.length})
                            </label>
                            <div className="flex flex-col gap-1">
                                {matchingVehicles.map(v => {
                                    const motor = v.motorization as Motorization;
                                    return (
                                        <button
                                            key={v.id}
                                            onClick={() => onSelectVehicle(v.id)}
                                            className="flex items-center gap-2 bg-gray-700 hover:bg-gray-600 rounded px-2 py-1 text-left transition-colors"
                                        >
                                            <span className="text-white text-xs flex-1">Véhicule #{v.id}</span>
                                            <span className={`text-xs ${MOTOR_COLORS[motor]}`}>{MOTOR_LABELS[motor]}</span>
                                        </button>
                                    );
                                })}
                            </div>
                        </div>
                    </div>
                );
            }
        }
    }

    else {
        content = (
            <p className="text-sm text-gray-400">
                Cliquez sur un <span className="text-blue-400">Habitation</span> ou un <span className="text-red-400">Travail</span> pour voir les véhicules associés.
            </p>
        );
    }

    return (
        <div className="flex-shrink-0 w-64 ml-3 bg-black rounded-[10px] overflow-hidden flex flex-col">
            <div className="flex items-center justify-between px-4 py-3 border-b border-gray-600">
                <span className="text-white font-medium text-sm">Waypoints</span>
                <div className="flex items-center gap-2">
                    {showCancel && (
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
            <div className="p-4 overflow-y-auto flex-1">
                {content}
            </div>
        </div>
    );
}
