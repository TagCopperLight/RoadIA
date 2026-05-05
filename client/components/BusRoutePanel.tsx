'use client';

import React, { useState, useImperativeHandle, forwardRef } from 'react';
import { useWs } from '@/app/websocket/websocket';
import { useEditMode, BusRoute } from './EditModeContext';
import { MapData } from './map/types';

const ROUTE_COLORS = ['#FF6B6B', '#4ECDC4', '#45B7D1', '#FFA07A', '#98D8C8', '#F7DC6F', '#95E1D3', '#F38181'];

interface BusRoutePanelProps {
    mapData: MapData | null;
}

interface BusRoutePanelHandle {
    onNodeClick: (nodeId: number) => void;
    getSelectedRouteId: () => number | null;
    getPendingStops: () => number[];
}

export const BusRoutePanel = forwardRef<BusRoutePanelHandle, BusRoutePanelProps>(function BusRoutePanel({ mapData }, ref) {
    const ws = useWs();
    const { busRoutes, setBusRoutes, selectedBusRoute, setSelectedBusRoute } = useEditMode();

    // States
    const [showCreateForm, setShowCreateForm] = useState(false);
    const [tempRouteName, setTempRouteName] = useState('');
    const [creatingStops, setCreatingStops] = useState<number[]>([]);
    const [editingRouteId, setEditingRouteId] = useState<number | null>(null);

    const nodeMap = new Map(mapData?.nodes.map(n => [n.id, n]) ?? []);

    // Start creating a new route
    const handleStartCreate = () => {
        setShowCreateForm(true);
        setTempRouteName('');
        setCreatingStops([]);
    };

    // Cancel creating
    const handleCancelCreate = () => {
        setShowCreateForm(false);
        setTempRouteName('');
        setCreatingStops([]);
    };

    // Add stop when node clicked during creation
    const handleAddStop = (nodeId: number) => {
        if (!creatingStops.includes(nodeId)) {
            setCreatingStops([...creatingStops, nodeId]);
        }
    };

    // Remove stop
    const handleRemoveStop = (index: number) => {
        setCreatingStops(creatingStops.filter((_, i) => i !== index));
    };

    // Save and create the route
    const handleSaveNewRoute = () => {
        if (creatingStops.length < 2) {
            alert('Route must have at least 2 stops (spawn and terminus)');
            return;
        }

        const newRoute: BusRoute = {
            id: Date.now(),
            name: tempRouteName.trim() || `Route ${busRoutes.length + 1}`,
            stops: creatingStops,
            color: ROUTE_COLORS[busRoutes.length % ROUTE_COLORS.length],
        };

        // Add to state and send to server
        setBusRoutes([...busRoutes, newRoute]);
        setSelectedBusRoute(newRoute.id);

        ws?.send('setBusRoute', {
            route_id: newRoute.id,
            route_name: newRoute.name,
            stop_node_ids: creatingStops,
        });

        // Reset
        setShowCreateForm(false);
        setTempRouteName('');
        setCreatingStops([]);
    };

    // Delete route
    const handleDeleteRoute = (routeId: number) => {
        setBusRoutes(busRoutes.filter((r: BusRoute) => r.id !== routeId));
        if (selectedBusRoute === routeId) {
            setSelectedBusRoute(null);
        }
        ws?.send('deleteBusRoute', { route_id: routeId });
    };

    // Expose methods for external use (map clicks)
    React.useImperativeHandle(ref, () => ({
        onNodeClick: (nodeId: number) => {
            if (showCreateForm) {
                handleAddStop(nodeId);
            }
        },
        getSelectedRouteId: () => editingRouteId,
        getPendingStops: () => creatingStops,
    }));

    // EMPTY STATE - No routes and not creating
    if (busRoutes.length === 0 && !showCreateForm) {
        return (
            <div className="flex flex-col h-full bg-black border-l border-gray-600">
                <div className="p-4 border-b border-gray-600 flex-shrink-0">
                    <h3 className="text-sm font-semibold text-white uppercase tracking-wide">Bus Routes</h3>
                </div>
                <div className="flex-1 flex flex-col items-center justify-center p-4 gap-4">
                    <p className="text-xs text-gray-400 text-center">
                        No bus routes yet
                    </p>
                    <button
                        onClick={handleStartCreate}
                        className="px-4 py-2 bg-blue-700 text-white rounded text-xs font-medium hover:bg-blue-600 transition"
                    >
                        + New Route
                    </button>
                </div>
            </div>
        );
    }

    // CREATE NEW ROUTE VIEW
    if (showCreateForm) {
        return (
            <div className="flex flex-col h-full bg-black border-l border-gray-600">
                <div className="p-4 border-b border-gray-600 flex-shrink-0">
                    <h3 className="text-sm font-semibold text-white uppercase tracking-wide">New Route</h3>
                    <button
                        onClick={handleCancelCreate}
                        className="text-xs text-gray-400 hover:text-gray-300 mt-2"
                    >
                        Back
                    </button>
                </div>

                <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-4">
                    {/* Route Name Input */}
                    <div className="flex flex-col gap-2">
                        <label className="text-xs text-gray-400 uppercase tracking-wide">Route Name</label>
                        <input
                            type="text"
                            placeholder="e.g., Route 1, Downtown Express"
                            value={tempRouteName}
                            onChange={(e) => setTempRouteName(e.target.value)}
                            className="px-3 py-2 bg-gray-800 text-white rounded text-sm border border-gray-700 focus:border-blue-500 outline-none"
                            autoFocus
                        />
                    </div>

                    {/* Stops List */}
                    <div className="flex flex-col gap-2">
                        <p className="text-xs text-gray-400 uppercase tracking-wide">
                            Stops ({creatingStops.length})
                        </p>
                        {creatingStops.length === 0 ? (
                            <p className="text-xs text-gray-500 italic text-center py-4">
                                Click nodes on the map to add stops
                            </p>
                        ) : (
                            <div className="flex flex-col gap-1 bg-gray-900 p-2 rounded max-h-64 overflow-y-auto">
                                {creatingStops.map((nodeId, idx) => {
                                    const node = nodeMap.get(nodeId);
                                    const nodeName = node ? `${node.kind}` : `Node ${nodeId}`;
                                    let label = '';
                                    if (idx === 0) label = '🚌 SPAWN';
                                    else if (idx === creatingStops.length - 1) label = '🎯 TERMINUS';
                                    else label = '📍 WAYPOINT';

                                    return (
                                        <div
                                            key={idx}
                                            className="flex justify-between items-center px-2 py-1 bg-gray-800 rounded text-xs text-gray-200"
                                        >
                                            <span>
                                                <strong>{label}</strong> Node {nodeId} ({nodeName})
                                            </span>
                                            <button
                                                onClick={() => handleRemoveStop(idx)}
                                                className="text-red-500 hover:text-red-400 font-bold"
                                            >
                                                ✕
                                            </button>
                                        </div>
                                    );
                                })}
                            </div>
                        )}
                    </div>

                    <div className="text-xs text-gray-400 bg-gray-900 p-2 rounded">
                        <p className="mb-1">ℹ️ <strong>Instructions:</strong></p>
                        <ul className="list-disc list-inside space-y-1">
                            <li>1st click = Spawn location (🚌)</li>
                            <li>2nd to n-1 clicks = Waypoints (📍)</li>
                            <li>Last click = Terminus (🎯)</li>
                        </ul>
                    </div>
                </div>

                {/* Action Buttons */}
                <div className="flex gap-2 p-4 border-t border-gray-600 flex-shrink-0">
                    <button
                        onClick={handleCancelCreate}
                        className="flex-1 px-3 py-2 bg-gray-700 text-white rounded text-xs font-medium hover:bg-gray-600 transition"
                    >
                        Cancel
                    </button>
                    <button
                        onClick={handleSaveNewRoute}
                        disabled={creatingStops.length < 2}
                        className="flex-1 px-3 py-2 bg-green-700 text-white rounded text-xs font-medium hover:bg-green-600 transition disabled:opacity-50 disabled:cursor-not-allowed"
                        title={creatingStops.length < 2 ? "Add at least 2 stops (spawn + terminus)" : "Create route"}
                    >
                        Create Route
                    </button>
                </div>
            </div>
        );
    }

    // ROUTES LIST VIEW
    return (
        <div className="flex flex-col h-full bg-black border-l border-gray-600">
            <div className="p-4 border-b border-gray-600 flex-shrink-0">
                <h3 className="text-sm font-semibold text-white uppercase tracking-wide">Bus Routes</h3>
            </div>

            <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-2">
                {busRoutes.map((route: BusRoute) => (
                    <div
                        key={route.id}
                        className={`p-3 bg-gray-900 rounded border-l-4 transition ${
                            selectedBusRoute === route.id ? 'border-opacity-100' : 'border-opacity-50 hover:border-opacity-75'
                        }`}
                        style={{ borderLeftColor: route.color }}
                    >
                        <div className="flex items-start justify-between gap-2">
                            <div className="flex-1 min-w-0">
                                <h4 className="text-sm font-semibold text-white truncate">{route.name}</h4>
                                <p className="text-xs text-gray-400 mt-1">
                                    {route.stops.length} stop{route.stops.length !== 1 ? 's' : ''}
                                </p>
                            </div>
                        </div>
                        <div className="flex gap-2 mt-3">
                            <button
                                onClick={() => handleDeleteRoute(route.id)}
                                className="flex-1 px-2 py-1 bg-red-700 text-white rounded text-xs font-medium hover:bg-red-600 transition"
                            >
                                Delete
                            </button>
                        </div>
                    </div>
                ))}
            </div>

            <div className="p-4 border-t border-gray-600 flex-shrink-0">
                <button
                    onClick={handleStartCreate}
                    className="w-full px-3 py-2 bg-green-700 text-white rounded text-xs font-medium hover:bg-green-600 transition"
                >
                    + New Route
                </button>
            </div>
        </div>
    );
});

export default BusRoutePanel;

