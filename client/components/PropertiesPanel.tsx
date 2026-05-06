'use client';

import { useState } from 'react';
import { SelectedElement } from './EditModeContext';
import { MapData, MapNode, MapEdge, InternalLane } from './map/types';

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

function NodePanel({ node, onSendPacket, onClose }: { node: MapNode; onSendPacket: PropsPanelProps['onSendPacket']; onClose: () => void }) {
    const [kind, setKind] = useState(node.kind);
    const [prevKind, setPrevKind] = useState(node.kind);
    if (node.kind !== prevKind) {
        setPrevKind(node.kind);
        setKind(node.kind);
    }

    const handleKindChange = (newKind: MapNode['kind']) => {
        setKind(newKind);
        onSendPacket('updateNode', { id: node.id, kind: newKind });
    };

    const lanes = node.internal_lanes ?? [];

    const handleDelete = () => {
        onSendPacket('deleteNode', { id: node.id });
        onClose();
    };

    return (
        <div className="flex flex-col gap-3">
            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">ID</label>
                <span className="text-white text-sm">{node.id}</span>
            </div>

            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">Kind</label>
                <select
                    value={kind}
                    onChange={e => handleKindChange(e.target.value as MapNode['kind'])}
                    className="bg-black text-white text-sm rounded px-2 py-1 border border-gray-600 focus:outline-none focus:border-gray-200"
                >
                    <option value="Intersection">Intersection</option>
                    <option value="Habitation">Habitation</option>
                    <option value="Workplace">Workplace</option>
                </select>
            </div>

            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">Traffic Light</label>
                <span className="text-white text-sm">{node.has_traffic_light ? 'Yes' : 'No'}</span>
            </div>

            <div className="flex flex-col gap-1">
                <label className="text-xs text-gray-400 uppercase tracking-wide">Radius</label>
                <span className="text-white text-sm">{node.radius.toFixed(1)} m</span>
            </div>

            {lanes.length > 0 && (
                <div className="flex flex-col gap-2">
                    <label className="text-xs text-gray-400 uppercase tracking-wide">
                        Internal Lanes ({lanes.length})
                    </label>
                    <div className="flex flex-col gap-1 max-h-48 overflow-y-auto">
                        {lanes.map((lane: InternalLane) => (
                            <div
                                key={lane.id}
                                className="flex items-center gap-2 bg-gray-700 rounded px-2 py-1"
                            >
                                <span
                                    className="w-2.5 h-2.5 rounded-full flex-shrink-0"
                                    style={{ backgroundColor: LINK_TYPE_COLORS[lane.link_type] ?? '#888' }}
                                />
                                <span className="text-white text-xs flex-1">{lane.link_type}</span>
                                <span className="text-gray-400 text-xs">#{lane.id}</span>
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
