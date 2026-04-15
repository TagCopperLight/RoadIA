'use client';

import React, { useState, useImperativeHandle, forwardRef, useCallback } from 'react';
import { useWs } from '@/app/websocket/websocket';

interface VehicleInfo {
  id: number;
  origin_node_id: number;
  dest_node_id: number;
  vehicle_type: string;
}

interface WaypointPanelProps {
  vehicles: VehicleInfo[];
}

export const WaypointPanel = forwardRef(function WaypointPanel({
  vehicles,
}: WaypointPanelProps, ref) {
  const ws = useWs();
  
  // Panel state
  const [clickedNodeId, setClickedNodeId] = useState<number | null>(null);
  const [selectedVehicleId, setSelectedVehicleId] = useState<number | null>(null);
  const [pendingWaypoints, setPendingWaypoints] = useState<number[]>([]);

  // Add waypoint when node is clicked
  const handleAddWaypoint = useCallback((nodeId: number) => {
    if (!selectedVehicleId) return;
    
    // Don't add if it's the current destination
    const vehicle = vehicles.find(v => v.id === selectedVehicleId);
    if (vehicle && nodeId === vehicle.dest_node_id) return;
    
    // Don't add if already in waypoints
    if (pendingWaypoints.includes(nodeId)) return;
    
    setPendingWaypoints([...pendingWaypoints, nodeId]);
  }, [selectedVehicleId, pendingWaypoints, vehicles]);

  // Remove waypoint from list
  const handleRemoveWaypoint = (index: number) => {
    setPendingWaypoints(pendingWaypoints.filter((_, i) => i !== index));
  };

  // Apply waypoints to backend
  const handleApply = () => {
    if (!selectedVehicleId) return;
    
    ws?.send('addWaypoints', {
      vehicle_id: selectedVehicleId,
      waypoint_node_ids: pendingWaypoints,
    });
    
    // Reset UI
    setSelectedVehicleId(null);
    setClickedNodeId(null);
    setPendingWaypoints([]);
  };

  // Cancel editing
  const handleCancel = () => {
    setSelectedVehicleId(null);
    setPendingWaypoints([]);
  };

  // Expose methods for external use (map clicks)
  useImperativeHandle(ref, () => ({
    onNodeClick: (nodeId: number) => {
      // If already selecting a vehicle, add as waypoint
      if (selectedVehicleId) {
        handleAddWaypoint(nodeId);
      } else {
        // Otherwise, show vehicles for this node
        setClickedNodeId(nodeId);
      }
    },
    getSelectedVehicleId: () => selectedVehicleId,
    getPendingWaypoints: () => pendingWaypoints,
  }));

  // Get vehicles for clicked node (origin or destination)
  const vehiclesForNode = clickedNodeId
    ? vehicles.filter(v => v.origin_node_id === clickedNodeId)
    : [];

  const selectedVehicle = selectedVehicleId
    ? vehicles.find(v => v.id === selectedVehicleId)
    : null;

  // EMPTY STATE
  if (!clickedNodeId && !selectedVehicleId) {
    return (
      <div className="flex flex-col h-full bg-black border-l border-gray-600">
        <div className="p-4 border-b border-gray-600 flex-shrink-0">
          <h3 className="text-sm font-semibold text-white uppercase tracking-wide">🚗 Waypoints</h3>
        </div>
        <div className="flex-1 flex items-center justify-center p-4">
          <p className="text-xs text-gray-400 text-center">
            Click a node on the map to select a vehicle
          </p>
        </div>
      </div>
    );
  }

  // VEHICLE SELECTION VIEW
  if (!selectedVehicleId && vehiclesForNode.length > 0) {
    return (
      <div className="flex flex-col h-full bg-black border-l border-gray-600">
        <div className="p-4 border-b border-gray-600 flex-shrink-0">
          <h3 className="text-sm font-semibold text-white uppercase tracking-wide">
            📍 Node {clickedNodeId} - Vehicles
          </h3>
          <button
            onClick={() => setClickedNodeId(null)}
            className="text-xs text-gray-400 hover:text-gray-300 mt-2"
          >
            ← Back
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-2">
          {vehiclesForNode.map(vehicle => (
            <button
              key={vehicle.id}
              onClick={() => setSelectedVehicleId(vehicle.id)}
              className="w-full text-left px-3 py-2 bg-gray-800 hover:bg-gray-700 text-white rounded text-sm transition"
            >
              <div className="font-medium">Vehicle #{vehicle.id}</div>
              <div className="text-xs text-gray-400">{vehicle.vehicle_type}</div>
              <div className="text-xs text-gray-500">
                → Node {vehicle.dest_node_id}
              </div>
            </button>
          ))}
        </div>
      </div>
    );
  }

  // VEHICLE DETAIL VIEW
  if (selectedVehicle) {
    return (
      <div className="flex flex-col h-full bg-black border-l border-gray-600">
        <div className="p-4 border-b border-gray-600 flex-shrink-0">
          <h3 className="text-sm font-semibold text-white uppercase tracking-wide">
            🚗 Vehicle #{selectedVehicle.id}
          </h3>
          <button
            onClick={handleCancel}
            className="text-xs text-gray-400 hover:text-gray-300 mt-2"
          >
            ← Back
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-4">
          {/* Vehicle Info */}
          <div className="flex flex-col gap-2">
            <div className="flex flex-col gap-1">
              <p className="text-xs text-gray-400 uppercase tracking-wide">Type</p>
              <p className="text-sm text-white">{selectedVehicle.vehicle_type}</p>
            </div>
            
            <div className="flex flex-col gap-1">
              <p className="text-xs text-gray-400 uppercase tracking-wide">Origin</p>
              <p className="text-sm text-green-400 font-mono">Node {selectedVehicle.origin_node_id}</p>
            </div>
            
            <div className="flex flex-col gap-1">
              <p className="text-xs text-gray-400 uppercase tracking-wide">Destination</p>
              <p className="text-sm text-red-400 font-mono">Node {selectedVehicle.dest_node_id}</p>
            </div>
          </div>

          {/* Waypoints List */}
          <div className="flex flex-col gap-2">
            <p className="text-xs text-gray-400 uppercase tracking-wide">Waypoints ({pendingWaypoints.length})</p>
            {pendingWaypoints.length === 0 ? (
              <p className="text-xs text-gray-500 italic">None - click nodes on map to add</p>
            ) : (
              <div className="flex flex-col gap-1 bg-gray-900 p-2 rounded">
                {pendingWaypoints.map((nodeId, idx) => (
                  <div
                    key={idx}
                    className="flex justify-between items-center px-2 py-1 bg-gray-800 rounded text-xs text-gray-200"
                  >
                    <span className="font-mono">
                      #{idx + 1} Node {nodeId}
                    </span>
                    <button
                      onClick={() => handleRemoveWaypoint(idx)}
                      className="text-red-500 hover:text-red-400 font-bold"
                    >
                      ✕
                    </button>
                  </div>
                ))}
              </div>
            )}
            <p className="text-xs text-gray-500 mt-2">
              💡 Click nodes on the map to add waypoints
            </p>
          </div>
        </div>

        {/* Action Buttons */}
        <div className="flex gap-2 p-4 border-t border-gray-600 flex-shrink-0">
          <button
            onClick={handleCancel}
            className="flex-1 px-3 py-2 bg-gray-700 text-white rounded text-xs font-medium hover:bg-gray-600 transition"
          >
            Cancel
          </button>
          <button
            onClick={handleApply}
            className="flex-1 px-3 py-2 bg-green-700 text-white rounded text-xs font-medium hover:bg-green-600 transition"
          >
            Apply
          </button>
        </div>
      </div>
    );
  }

  return null;
});
