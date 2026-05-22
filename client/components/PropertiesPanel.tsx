'use client';

import { useState } from 'react';
import { SelectedElement } from './EditModeContext';
import { MapData, MapNode, MapEdge, InternalLane, SignalPhase } from './map/types';

type LocalPhase = SignalPhase & { _key: number };
let _phaseKeyCounter = 0;
function toLocalPhases(serverPhases: SignalPhase[]): LocalPhase[] {
    return serverPhases.map(p => ({ ...p, _key: _phaseKeyCounter++ }));
}

const LINK_TYPE_COLORS: Record<string, string> = {
    Priority:     '#22c55e',
    Yield:        '#f59e0b',
    Stop:         '#ef4444',
    TrafficLight: '#3b82f6',
};

interface PropsPanelProps {
    selectedElement: NonNullable<SelectedElement>;
    mapData: MapData;
    onClose: () => void;
    onSendPacket: (id: string, data: Record<string, unknown>) => void;
}

function NodePanel({
    node,
    onSendPacket,
    onClose,
}: {
    node: MapNode;
    onSendPacket: PropsPanelProps['onSendPacket'];
    onClose: () => void;
}) {
    const [kind, setKind] = useState(node.kind);
    const [prevKind, setPrevKind] = useState(node.kind);
    if (node.kind !== prevKind) {
        setPrevKind(node.kind);
        setKind(node.kind);
    }

    const [phases, setPhases] = useState<LocalPhase[]>(() =>
        toLocalPhases(node.traffic_light_controller?.phases ?? [])
    );
    const [prevController, setPrevController] = useState(node.traffic_light_controller);
    if (node.traffic_light_controller !== prevController) {
        setPrevController(node.traffic_light_controller);
        setPhases(toLocalPhases(node.traffic_light_controller?.phases ?? []));
    }

    const handleKindChange = (newKind: MapNode['kind']) => {
        setKind(newKind);
        onSendPacket('updateNode', { id: node.id, kind: newKind });
    };

    const handleTrafficLightChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        onSendPacket('updateNode', { id: node.id, kind: node.kind, has_traffic_light: e.target.checked });
    };

    const handleInternalLaneTypeChange = (internal_lane_id: number, link_type: string) => {
        onSendPacket('updateInternalLane', { intersection_id: node.id, internal_lane_id, link_type });
    };

    const lanes = node.internal_lanes ?? [];
    const movements = lanes.filter(lane => lane.link_id !== undefined);

    const toggleLinkInPhase = (phaseIndex: number, linkId: number) => {
        setPhases(prev => prev.map((phase, idx) => {
            if (idx !== phaseIndex) return phase;
            const exists = phase.green_link_ids.includes(linkId);
            const newIds = exists
                ? phase.green_link_ids.filter(id => id !== linkId)
                : [...phase.green_link_ids, linkId];
            return { ...phase, green_link_ids: newIds };
        }));
    };

    const addPhase = () => {
        setPhases(prev => [
            ...prev,
            {
                _key: _phaseKeyCounter++,
                green_link_ids: [],
                green_duration: 10,
                yellow_duration: 3,
            }
        ]);
    };

    const deletePhase = (phaseIndex: number) => {
        setPhases(prev => prev.filter((_, idx) => idx !== phaseIndex));
    };

    const handleDurationChange = (phaseIndex: number, key: 'green_duration' | 'yellow_duration', value: number) => {
        setPhases(prev => prev.map((phase, idx) => {
            if (idx !== phaseIndex) return phase;
            return { ...phase, [key]: value };
        }));
    };

    const handleSaveTrafficLight = () => {
        const serverPhases: SignalPhase[] = phases.map(({ _key: _, ...p }) => p);
        onSendPacket('updateTrafficLight', { intersection_id: node.id, phases: serverPhases });
    };

    const handleDelete = () => {
        onSendPacket('deleteNode', { id: node.id });
        onClose();
    };

    return (
        <div className="flex flex-col gap-3">
            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">ID</label>
                <span className="text-white text-sm font-semibold">{node.id}</span>
            </div>

            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">Kind</label>
                <select
                    value={kind}
                    onChange={e => handleKindChange(e.target.value as MapNode['kind'])}
                    className="bg-black text-white text-sm rounded px-2 py-1 border border-gray-600 focus:outline-none focus:border-gray-200 cursor-pointer"
                >
                    <option value="Intersection">Intersection</option>
                    <option value="Habitation">Habitation</option>
                    <option value="Workplace">Workplace</option>
                </select>
            </div>

            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">Traffic Light</label>
                <div className="flex items-center gap-2">
                    <input
                        type="checkbox"
                        checked={node.has_traffic_light || false}
                        onChange={handleTrafficLightChange}
                        className="h-4 w-4 rounded border-gray-600 bg-black text-white focus:ring-0 cursor-pointer accent-white"
                    />
                    <span className="text-white text-xs">{node.has_traffic_light ? 'Enabled' : 'Disabled'}</span>
                </div>
            </div>

            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">Radius</label>
                <span className="text-white text-sm">{node.radius.toFixed(1)} m</span>
            </div>

            {/* Traffic Light Config Panel */}
            {node.has_traffic_light && (
                <div className="flex flex-col gap-3 border-t border-gray-600 pt-3">
                    <div className="flex items-center justify-between">
                        <label className="text-xs text-gray-400 uppercase tracking-wide font-semibold">Phase Editor</label>
                        <button
                            onClick={addPhase}
                            className="bg-black hover:bg-gray-800 text-white text-xs rounded px-2.5 py-1 border border-gray-600 transition-colors"
                        >
                            + Phase
                        </button>
                    </div>

                    {phases.length === 0 ? (
                        <div className="text-xs text-gray-500 text-center py-4 border border-dashed border-gray-600 rounded">
                            No phases configured. Click &quot;+ Phase&quot;.
                        </div>
                    ) : (
                        <div className="flex flex-col gap-3 max-h-72 overflow-y-auto pr-1">
                            {phases.map((phase, phaseIdx) => (
                                <div key={phase._key} className="border border-gray-600 rounded p-2.5 flex flex-col gap-2 relative bg-black">
                                    <div className="flex justify-between items-center">
                                        <span className="text-xs font-bold text-white">Phase #{phaseIdx + 1}</span>
                                        <button
                                            onClick={() => deletePhase(phaseIdx)}
                                            className="text-red-500 hover:text-red-400 text-xs transition-colors hover:underline"
                                            title="Delete Phase"
                                        >
                                            Delete
                                        </button>
                                    </div>

                                    {/* Durations */}
                                    <div className="grid grid-cols-2 gap-2">
                                        <div className="flex flex-col gap-1">
                                            <span className="text-[10px] text-gray-400 uppercase tracking-wide">Green (s)</span>
                                            <input
                                                type="number"
                                                min={1}
                                                max={120}
                                                value={phase.green_duration}
                                                onChange={e => handleDurationChange(phaseIdx, 'green_duration', Math.max(1, parseFloat(e.target.value) || 1))}
                                                className="bg-black text-white text-xs rounded px-2 py-1 border border-gray-600 focus:outline-none focus:border-gray-200 w-full"
                                            />
                                        </div>
                                        <div className="flex flex-col gap-1">
                                            <span className="text-[10px] text-gray-400 uppercase tracking-wide">Yellow (s)</span>
                                            <input
                                                type="number"
                                                min={1}
                                                max={10}
                                                value={phase.yellow_duration}
                                                onChange={e => handleDurationChange(phaseIdx, 'yellow_duration', Math.max(1, parseFloat(e.target.value) || 1))}
                                                className="bg-black text-white text-xs rounded px-2 py-1 border border-gray-600 focus:outline-none focus:border-gray-200 w-full"
                                            />
                                        </div>
                                    </div>

                                    {/* Allowed Movements */}
                                    <div className="flex flex-col gap-1.5">
                                        <span className="text-[10px] text-gray-400 uppercase tracking-wide">Green Turns</span>
                                        {movements.length === 0 ? (
                                            <span className="text-[10px] text-gray-500 italic">No turns detected. Add roads first.</span>
                                        ) : (
                                            <div className="flex flex-col gap-1 max-h-36 overflow-y-auto border border-gray-600 rounded p-1.5 bg-black">
                                                {movements.map(lane => {
                                                    const linkId = lane.link_id!;
                                                    const isActive = phase.green_link_ids.includes(linkId);
                                                    return (
                                                        <button
                                                            key={lane.id}
                                                            type="button"
                                                            onClick={() => toggleLinkInPhase(phaseIdx, linkId)}
                                                            className={`flex items-center justify-between text-left px-2 py-1 rounded text-[10px] transition-colors border ${
                                                                isActive
                                                                    ? 'bg-black border-green-600 text-green-400 font-medium'
                                                                    : 'bg-black border-transparent text-gray-400 hover:border-gray-600'
                                                            }`}
                                                        >
                                                            <span className="truncate">
                                                                Road #{lane.from_road_id} ➔ #{lane.to_road_id}
                                                            </span>
                                                            {isActive && (
                                                                <span className="w-1.5 h-1.5 rounded-full bg-green-500 flex-shrink-0 ml-1" />
                                                            )}
                                                        </button>
                                                    );
                                                })}
                                            </div>
                                        )}
                                    </div>
                                </div>
                            ))}
                        </div>
                    )}

                    <button
                        onClick={handleSaveTrafficLight}
                        disabled={phases.length === 0}
                        className="w-full bg-black hover:bg-gray-800 text-white text-xs font-semibold rounded py-2 border border-gray-600 transition-colors disabled:opacity-30 disabled:hover:bg-black disabled:cursor-not-allowed"
                    >
                        Save Timing & Phases
                    </button>
                </div>
            )}

            {/* Default Internal Lanes (Only when traffic light is disabled) */}
            {!node.has_traffic_light && lanes.length > 0 && (
                <div className="flex flex-col gap-2 border-t border-gray-600 pt-3">
                    <label className="text-xs text-gray-400 uppercase tracking-wide">
                        Internal Lanes ({lanes.length})
                    </label>
                    <div className="flex flex-col gap-1 max-h-48 overflow-y-auto">
                        {lanes.map((lane: InternalLane) => (
                            <div
                                key={lane.id}
                                className="flex items-center gap-2 bg-black rounded px-2 py-1.5 border border-gray-600"
                            >
                                <span
                                    className="w-2 h-2 rounded-full flex-shrink-0"
                                    style={{ backgroundColor: LINK_TYPE_COLORS[lane.link_type] ?? '#888' }}
                                />
                                <span className="text-gray-400 text-xs w-6">#{lane.id}</span>
                                <select
                                    value={lane.link_type}
                                    onChange={(e) => handleInternalLaneTypeChange(lane.id, e.target.value)}
                                    className="flex-1 bg-black text-white text-xs rounded px-1.5 py-0.5 border border-gray-600 focus:outline-none focus:border-gray-200"
                                >
                                    <option value="Priority">Priority</option>
                                    <option value="Yield">Yield</option>
                                    <option value="Stop">Stop</option>
                                </select>
                            </div>
                        ))}
                    </div>
                </div>
            )}

            <button
                onClick={handleDelete}
                className="mt-1 bg-red-900 hover:bg-red-800 text-white text-xs rounded px-3 py-1.5 border border-red-700 transition-colors w-full"
            >
                Delete Node
            </button>
        </div>
    );
}

function RoadPanel({
    canonical,
    reverse,
    onSendPacket,
    onClose,
}: {
    canonical: MapEdge;
    reverse?: MapEdge;
    onSendPacket: PropsPanelProps['onSendPacket'];
    onClose: () => void;
}) {
    // Display in km/h; backend uses m/s
    const toKmh = (ms: number) => Math.round(ms * 3.6);
    const toMs = (kmh: number) => kmh / 3.6;

    const [speedKmh, setSpeedKmh] = useState(toKmh(canonical.speed_limit));
    const [laneCount, setLaneCount] = useState(canonical.lane_count);
    const [prevSpeedLimit, setPrevSpeedLimit] = useState(canonical.speed_limit);
    const [prevLaneCount, setPrevLaneCount] = useState(canonical.lane_count);
    if (canonical.speed_limit !== prevSpeedLimit) {
        setPrevSpeedLimit(canonical.speed_limit);
        setSpeedKmh(toKmh(canonical.speed_limit));
    }
    if (canonical.lane_count !== prevLaneCount) {
        setPrevLaneCount(canonical.lane_count);
        setLaneCount(canonical.lane_count);
    }

    const handleSpeedBlur = () => {
        const ms = toMs(speedKmh);
        onSendPacket('updateRoad', { id: canonical.id, speed_limit: ms });
        if (reverse) {
            onSendPacket('updateRoad', { id: reverse.id, speed_limit: ms });
        }
    };

    const handleLaneCountBlur = () => {
        const count = Math.max(1, Math.min(8, laneCount));
        setLaneCount(count);
        onSendPacket('updateRoad', { id: canonical.id, speed_limit: toMs(speedKmh), lane_count: count });
        if (reverse) {
            onSendPacket('updateRoad', { id: reverse.id, speed_limit: toMs(speedKmh), lane_count: count });
        }
    };

    const handleMakeOneWay = () => {
        if (reverse) {
            onSendPacket('deleteRoad', { id: reverse.id });
        }
    };

    const handleMakeTwoWay = () => {
        onSendPacket('addRoad', {
            from_id: canonical.to,
            to_id: canonical.from,
            lane_count: canonical.lane_count,
            speed_limit: canonical.speed_limit,
        });
    };

    const handleSwapDirection = () => {
        onSendPacket('deleteRoad', { id: canonical.id });
        onSendPacket('addRoad', {
            from_id: canonical.to,
            to_id: canonical.from,
            lane_count: canonical.lane_count,
            speed_limit: canonical.speed_limit,
        });
        onClose();
    };

    const handleDelete = () => {
        onSendPacket('deleteRoad', { id: canonical.id });
        if (reverse) onSendPacket('deleteRoad', { id: reverse.id });
        onClose();
    };

    return (
        <div className="flex flex-col gap-3">
            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">Direction</label>
                <span className="text-white text-sm">
                    Node {canonical.from} → Node {canonical.to}
                </span>
            </div>

            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">Lanes per direction</label>
                <input
                    type="number"
                    min={1}
                    max={8}
                    value={laneCount}
                    onChange={e => setLaneCount(Number(e.target.value))}
                    onBlur={handleLaneCountBlur}
                    onKeyDown={e => { if (e.key === 'Enter') handleLaneCountBlur(); }}
                    className="bg-black text-white text-sm rounded px-2 py-1 border border-gray-600 focus:outline-none focus:border-gray-200 w-24"
                />
            </div>

            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">Length</label>
                <span className="text-white text-sm">{canonical.length.toFixed(0)} m</span>
            </div>

            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">Speed Limit (km/h)</label>
                <input
                    type="number"
                    min={4}
                    max={150}
                    value={speedKmh}
                    onChange={e => setSpeedKmh(Number(e.target.value))}
                    onBlur={handleSpeedBlur}
                    onKeyDown={e => { if (e.key === 'Enter') handleSpeedBlur(); }}
                    className="bg-black text-white text-sm rounded px-2 py-1 border border-gray-600 focus:outline-none focus:border-gray-200 w-24"
                />
            </div>

            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">Direction</label>
                {reverse ? (
                    <button
                        onClick={handleMakeOneWay}
                        className="bg-black hover:bg-gray-800 text-white text-xs rounded px-3 py-1.5 border border-gray-600 transition-colors text-left"
                    >
                        Make one-way
                    </button>
                ) : (
                    <div className="flex flex-col gap-1">
                        <button
                            onClick={handleMakeTwoWay}
                            className="bg-black hover:bg-gray-800 text-white text-xs rounded px-3 py-1.5 border border-gray-600 transition-colors text-left"
                        >
                            Make two-way
                        </button>
                        <button
                            onClick={handleSwapDirection}
                            className="bg-black hover:bg-gray-800 text-white text-xs rounded px-3 py-1.5 border border-gray-600 transition-colors text-left"
                        >
                            Swap direction ⇄
                        </button>
                    </div>
                )}
            </div>

            <button
                onClick={handleDelete}
                className="mt-1 bg-red-900 hover:bg-red-800 text-white text-xs rounded px-3 py-1.5 border border-red-700 transition-colors w-full"
            >
                Delete Road
            </button>
        </div>
    );
}

export default function PropertiesPanel({ selectedElement, mapData, onClose, onSendPacket }: PropsPanelProps) {
    let title = '';
    let content: React.ReactNode = null;

    if (selectedElement.type === 'node') {
        const node = mapData.nodes.find(n => n.id === selectedElement.id);
        if (!node) return null;
        title = 'Intersection';
        content = <NodePanel node={node} onSendPacket={onSendPacket} onClose={onClose} />;
    } else {
        const canonical = mapData.edges.find(e => e.id === selectedElement.canonicalId);
        if (!canonical) return null;
        const reverse = selectedElement.reverseId != null
            ? mapData.edges.find(e => e.id === selectedElement.reverseId)
            : undefined;
        title = reverse ? 'Road (Two-way)' : 'Road (One-way)';
        content = <RoadPanel canonical={canonical} reverse={reverse} onSendPacket={onSendPacket} onClose={onClose} />;
    }

    return (
        <div className="flex-shrink-0 w-64 ml-3 bg-black rounded-[10px] overflow-hidden flex flex-col">
            {/* Header */}
            <div className="flex items-center justify-between px-4 py-3 border-b border-gray-600">
                <span className="text-white font-medium text-sm">{title}</span>
                <button
                    onClick={onClose}
                    className="text-gray-400 hover:text-white transition-colors text-lg leading-none"
                    title="Close"
                >
                    ×
                </button>
            </div>

            {/* Body */}
            <div className="p-4 overflow-y-auto flex-1">
                {content}
            </div>
        </div>
    );
}
