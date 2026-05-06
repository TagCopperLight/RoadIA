'use client';

import React, { useState, useImperativeHandle, forwardRef, useCallback } from 'react';
import { useWs } from '@/app/websocket/websocket';
import { MapData } from './map/types';

interface VehicleInfo {
  id: number;
  origin_node_id: number;
  dest_node_id: number;
  vehicle_type: string;
}

interface WaypointPanelProps {
  vehicles: VehicleInfo[];
  mapData?: MapData | null;
}

export const WaypointPanel = forwardRef(function WaypointPanel({
  vehicles,
  mapData,
}: WaypointPanelProps, ref) {
  const ws = useWs();
  
  // Panel state
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
      }
    },
    getSelectedVehicleId: () => selectedVehicleId,
    getPendingWaypoints: () => pendingWaypoints,
  }));

  // EMPTY STATE
  if (vehicles.length === 0) {
    return (
      <div className="flex flex-col h-full bg-black border-l border-gray-600">
        <div className="p-4 border-b border-gray-600 flex-shrink-0">
          <h3 className="text-sm font-semibold text-white uppercase tracking-wide">Waypoints</h3>
        </div>
        <div className="flex-1 flex flex-col items-center justify-center p-4">
          <p className="text-xs text-gray-400 text-center">
            No vehicles in simulation
          </p>
        </div>
      </div>
    );
  }

  // SELECT VEHICLE VIEW
  if (!selectedVehicleId) {
    return (
      <div className="flex flex-col h-full bg-black border-l border-gray-600">
        <div className="p-4 border-b border-gray-600 flex-shrink-0">
          <h3 className="text-sm font-semibold text-white uppercase tracking-wide">Waypoints</h3>
        </div>

        <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-2">
          {vehicles.map(vehicle => (
            <button
              key={vehicle.id}
              onClick={() => {
                setSelectedVehicleId(vehicle.id);
              }}
              className="w-full text-left p-3 bg-gray-900 rounded border border-gray-700 hover:border-blue-500 transition text-xs"
            >
              <div className="font-semibold text-white">Vehicle {vehicle.id}</div>
              <div className="text-gray-400 mt-1">
                Type: {vehicle.vehicle_type}
              </div>
            </button>
          ))}
        </div>
      </div>
    );
  }

  // EDITING VIEW
  const selectedVehicle = vehicles.find(v => v.id === selectedVehicleId);
  
  return (
    <div className="flex flex-col h-full bg-black border-l border-gray-600">
      <div className="p-4 border-b border-gray-600 flex-shrink-0">
        <h3 className="text-sm font-semibold text-white uppercase tracking-wide">
          Vehicle {selectedVehicleId}
        </h3>
        <button
          onClick={() => setSelectedVehicleId(null)}
          className="text-xs text-gray-400 hover:text-gray-300 mt-2"
        >
          ← Back
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-4 flex flex-col gap-4">
        <div>
          <p className="text-xs text-gray-400 uppercase tracking-wide mb-2">
            Destination
          </p>
          {selectedVehicle && mapData && (
            <div className="text-sm text-white bg-gray-900 p-2 rounded">
              Node {selectedVehicle.dest_node_id}
            </div>
          )}
        </div>

        <div>
          <p className="text-xs text-gray-400 uppercase tracking-wide mb-2">
            Waypoints ({pendingWaypoints.length})
          </p>
          {pendingWaypoints.length === 0 ? (
            <p className="text-xs text-gray-500 italic text-center py-4">
              Click nodes on the map to add waypoints
            </p>
          ) : (
            <div className="space-y-2">
              {pendingWaypoints.map((nodeId, i) => (
                <div
                  key={i}
                  className="flex items-center justify-between bg-gray-900 p-2 rounded text-sm"
                >
                  <span className="text-white">{i + 1}. Node {nodeId}</span>
                  <button
                    onClick={() => handleRemoveWaypoint(i)}
                    className="text-red-500 hover:text-red-400 text-xs font-bold"
                  >
                    ✕
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>

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
});
