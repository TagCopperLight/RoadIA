'use client';

import React, { useState, useImperativeHandle, forwardRef, useCallback } from 'react';
import { useWs } from '@/app/websocket/websocket';
import { MapData } from './map/types';
import { useEditMode, BusRoute } from './EditModeContext';

const ROUTE_COLORS = ['#FF6B6B', '#4ECDC4', '#45B7D1', '#FFA07A', '#98D8C8', '#F7DC6F', '#95E1D3', '#F38181'];

interface BusRoutePanelProps {
    mapData: MapData | null;
}

interface BusRoutePanelHandle {
    onNodeClick: (nodeId: number) => void;
    getSelectedRouteId: () => number | null;
    getPendingStops: () => number[];
}

export const BusRoutePanel = forwardRef(function BusRoutePanel({ mapData,}: BusRoutePanelProps, ref) {
    const ws = useWs();
    const { busRoutes, setBusRoutes, selectedBusRoute, setSelectedBusRoute } = useEditMode();

    // Local state for editing
    const [creatingNewRoute, setCreatingNewRoute] = useState(false);
    const [tempRouteName, setTempRouteName] = useState('');
    const [editingRouteId, setEditingRouteId] = useState<number | null>(null);
    const [pendingStops, setPendingStops] = useState<number[]>([]);

    const nodeMap = new Map(mapData?.nodes.map(n => [n.id, n]) ?? []);

    const handleCreateRoute = () => {
        const newRoute: BusRoute = {
            id: Date.now(),
            name: tempRouteName.trim() || `Route ${busRoutes.length + 1}`,
            stops: [],
            color: ROUTE_COLORS[busRoutes.length % ROUTE_COLORS.length],
        };
        setBusRoutes([...busRoutes, newRoute]);
        setSelectedBusRoute(newRoute.id);
        setEditingRouteId(newRoute.id);
        setPendingStops([]);
        setCreatingNewRoute(false);
        setTempRouteName('');
    };

    // Start editing a route
    const handleStartEdit = (routeId: number) => {
        setEditingRouteId(routeId);
        const route = busRoutes.find(r => r.id === routeId);
        if (route) {
            setPendingStops([...route.stops]);
            setSelectedBusRoute(routeId);
        }
    };

    // Cancel editing
    const handleCancelEdit = () => {
        setEditingRouteId(null);
        setPendingStops([]);
    };

    // Save route changes and send to server
    const handleSaveRoute = () => {
        if (editingRouteId === null) return;
        
        const updatedRoutes = busRoutes.map(r =>
            r.id === editingRouteId
                ? { ...r, stops: pendingStops }
                : r
        );
        setBusRoutes(updatedRoutes);

        // Send to server
        const route = updatedRoutes.find(r => r.id === editingRouteId);
        if (route && route.stops.length > 0) {
            ws?.send('setBusRoute', {
                route_id: route.id,
                route_name: route.name,
                stop_node_ids: route.stops,
            });
        }

        handleCancelEdit();
    };

    // Delete a route
    const handleDeleteRoute = (routeId: number) => {
        setBusRoutes(busRoutes.filter(r => r.id !== routeId));
        if (selectedBusRoute === routeId) {
            setSelectedBusRoute(null);
        }
        if (editingRouteId === routeId) {
            handleCancelEdit();
        }
        
        // Send deletion to server
        ws?.send('deleteBusRoute', {
            route_id: routeId,
        });
    };

    // Add stop when node is clicked (if in edit mode)
    const handleAddStop = (nodeId: number) => {
        if (editingRouteId === null) return;
        
        // Don't add if already in stops
        if (pendingStops.includes(nodeId)) return;
        
        setPendingStops([...pendingStops, nodeId]);
    };

    // Remove stop
    const handleRemoveStop = (stopIndex: number) => {
        setPendingStops(pendingStops.filter((_, i) => i !== stopIndex));
    };

    // Expose methods for external use
    useImperativeHandle(ref, () => ({
        onNodeClick: (nodeId: number) => {
            if (editingRouteId !== null) {
                handleAddStop(nodeId);
            }
        },
        getSelectedRouteId: () => editingRouteId,
        getPendingStops: () => pendingStops,
    }));

    // EMPTY STATE
    if (busRoutes.length === 0 && !creatingNewRoute) {
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
                        onClick={() => setCreatingNewRoute(true)}
                        className="px-4 py-2 bg-blue-700 text-white rounded text-xs font-medium hover:bg-blue-600 transition"
                    >
                        + New Route
                    </button>
                </div>
            </div>
        );
    }

    // CREATE NEW ROUTE VIEW
    if (creatingNewRoute) {
        return (
            <div className="flex flex-col h-full bg-black border-l border-gray-600">
                <div className="p-4 border-b border-gray-600 flex-shrink-0">
                    <h3 className="text-sm font-semibold text-white uppercase tracking-wide">New Route</h3>
                    <button
                        onClick={() => {
                            setCreatingNewRoute(false);
                            setTempRouteName('');
                        }}
                        className="text-xs text-gray-400 hover:text-gray-300 mt-2"
                    >
                        Back
                    </button>
                </div>
                <div className="flex-1 flex flex-col gap-4 p-4">
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
                </div>
                <div className="flex gap-2 p-4 border-t border-gray-600 flex-shrink-0">
                    <button
                        onClick={() => {
                            setCreatingNewRoute(false);
                            setTempRouteName('');
                        }}
                        className="flex-1 px-3 py-2 bg-gray-700 text-white rounded text-xs font-medium hover:bg-gray-600 transition"
                    >
                        Cancel
                    </button>
                    <button
                        onClick={handleCreateRoute}
                        className="flex-1 px-3 py-2 bg-green-700 text-white rounded text-xs font-medium hover:bg-green-600 transition"
                    >
                        Create
                    </button>
                </div>
            </div>
        );
    }

    // ROUTES LIST VIEW
    if (!editingRouteId) {
        return (
            <div className="flex flex-col h-full bg-black border-l border-gray-600">
                <div className="p-4 border-b border-gray-600 flex-shrink-0">
                    <h3 className="text-sm font-semibold text-white uppercase tracking-wide">Bus Routes</h3>
                </div>

                <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-2">
                    {busRoutes.map(route => (
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
                                    onClick={() => handleStartEdit(route.id)}
                                    className="flex-1 px-2 py-1 bg-blue-700 text-white rounded text-xs font-medium hover:bg-blue-600 transition"
                                >
                                    Edit
                                </button>
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
                        onClick={() => setCreatingNewRoute(true)}
                        className="w-full px-3 py-2 bg-green-700 text-white rounded text-xs font-medium hover:bg-green-600 transition"
                    >
                        + New Route
                    </button>
                </div>
            </div>
        );
    }

    // EDIT ROUTE VIEW
    const editingRoute = busRoutes.find(r => r.id === editingRouteId);
    if (editingRoute && editingRouteId !== null) {
        return (
            <div className="flex flex-col h-full bg-black border-l border-gray-600">
                <div className="p-4 border-b border-gray-600 flex-shrink-0">
                    <h3 className="text-sm font-semibold text-white uppercase tracking-wide">
                        Editing: {editingRoute.name}
                    </h3>
                    <button
                        onClick={handleCancelEdit}
                        className="text-xs text-gray-400 hover:text-gray-300 mt-2"
                    >
                        Back
                    </button>
                </div>

                <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-4">
                    {/* Route Info */}
                    <div className="flex flex-col gap-2">
                        <p className="text-xs text-gray-400 uppercase tracking-wide">Stops ({pendingStops.length})</p>
                        {pendingStops.length === 0 ? (
                            <p className="text-xs text-gray-500 italic text-center py-4">
                                Click nodes on the map to add stops
                            </p>
                        ) : (
                            <div className="flex flex-col gap-1 bg-gray-900 p-2 rounded max-h-64 overflow-y-auto">
                                {pendingStops.map((stopNodeId, idx) => {
                                    const node = nodeMap.get(stopNodeId);
                                    const nodeName = node ? `Node ${node.id} (${node.kind})` : `Node ${stopNodeId}`;
                                    return (
                                        <div
                                            key={idx}
                                            className="flex justify-between items-center px-2 py-1 bg-gray-800 rounded text-xs text-gray-200 hover:bg-gray-700 transition"
                                        >
                                            <span className="font-mono">
                                                #{idx + 1} {nodeName}
                                            </span>
                                            <button
                                                onClick={() => handleRemoveStop(idx)}
                                                className="text-red-500 hover:text-red-400 font-bold text-sm"
                                                title="Remove stop"
                                            >
                                                ✕
                                            </button>
                                        </div>
                                    );
                                })}
                            </div>
                        )}
                    </div>

                    <div className="text-xs text-gray-500 bg-gray-900 p-2 rounded">
                        <p className="mb-2">💡 <strong>Tip:</strong> Click on nodes in the map to add them as stops to this route.</p>
                        <p>Stops define the path buses will follow. The order matters!</p>
                    </div>
                </div>

                {/* Action Buttons */}
                <div className="flex gap-2 p-4 border-t border-gray-600 flex-shrink-0">
                    <button
                        onClick={handleCancelEdit}
                        className="flex-1 px-3 py-2 bg-gray-700 text-white rounded text-xs font-medium hover:bg-gray-600 transition"
                    >
                        Cancel
                    </button>
                    <button
                        onClick={handleSaveRoute}
                        disabled={pendingStops.length === 0}
                        className="flex-1 px-3 py-2 bg-green-700 text-white rounded text-xs font-medium hover:bg-green-600 transition disabled:opacity-50 disabled:cursor-not-allowed"
                        title={pendingStops.length === 0 ? "Add at least one stop" : "Save route"}
                    >
                        Save Route
                    </button>
                </div>
            </div>
        );
    }

    return null;
});

export default BusRoutePanel;

