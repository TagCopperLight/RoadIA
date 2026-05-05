'use client';

import { useState, useEffect, useRef } from 'react';
import { MapContainer, TileLayer, useMapEvents, Rectangle } from 'react-leaflet';
import 'leaflet/dist/leaflet.css';
import { latLngBounds, LatLng, LatLngExpression, LatLngBounds } from 'leaflet';
import { useRouter } from 'next/navigation';

interface SelectionEventsProps {
  isCtrlPressed: boolean;
  onBoundsChange: (bounds: LatLngBounds | null) => void;
}

function SelectionEvents({ isCtrlPressed, onBoundsChange }: SelectionEventsProps) {
  const isSelectingRef = useRef(false);
  const startPointRef = useRef<LatLng | null>(null);
  const [previewBounds, setPreviewBounds] = useState<LatLngBounds | null>(null);

  const map = useMapEvents({
    mousedown(e) {
      if (isCtrlPressed || e.originalEvent.ctrlKey) return;
      isSelectingRef.current = true;
      startPointRef.current = e.latlng;
      setPreviewBounds(null);
      onBoundsChange(null);
    },
    mousemove(e) {
      if (!isSelectingRef.current || !startPointRef.current) return;
      setPreviewBounds(latLngBounds(startPointRef.current, e.latlng));
    },
    mouseup(e) {
      if (!isSelectingRef.current) return;
      isSelectingRef.current = false;
      if (startPointRef.current) {
        const newBounds = latLngBounds(startPointRef.current, e.latlng);
        setPreviewBounds(null);
        onBoundsChange(newBounds);
      }
    },
  });

  useEffect(() => {
    if (isCtrlPressed) {
      map.dragging.enable();
    } else {
      map.dragging.disable();
    }
  }, [map, isCtrlPressed]);

  return previewBounds ? (
    <Rectangle bounds={previewBounds} pathOptions={{ color: 'blue', weight: 2, fillOpacity: 0.2 }} />
  ) : null;
}

export default function MapSelection() {
  const router = useRouter();
  const [bounds, setBounds] = useState<LatLngBounds | null>(null);
  const [loading, setLoading] = useState(false);
  const [isCtrlPressed, setIsCtrlPressed] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Control') setIsCtrlPressed(true);
    };
    const handleKeyUp = (e: KeyboardEvent) => {
      if (e.key === 'Control') setIsCtrlPressed(false);
    };
    window.addEventListener('keydown', handleKeyDown);
    window.addEventListener('keyup', handleKeyUp);
    return () => {
      window.removeEventListener('keydown', handleKeyDown);
      window.removeEventListener('keyup', handleKeyUp);
    };
  }, []);

  const defaultCenter: LatLngExpression = [48.7323, -3.4589];
  const defaultZoom = 13;

  const handleImport = async () => {
    if (!bounds) return;
    setError(null);

    const distanceMeters = bounds.getSouthWest().distanceTo(bounds.getNorthEast());
    const MAX_DIAGONAL_METERS = 10000;
    if (distanceMeters > MAX_DIAGONAL_METERS) {
      setError(`La zone sélectionnée est trop grande (${(distanceMeters / 1000).toFixed(1)} km de diagonale). Veuillez sélectionner une zone plus petite (diagonale max : ${MAX_DIAGONAL_METERS / 1000} km).`);
      return;
    }

    setLoading(true);
    try {
      const min_lat = bounds.getSouthWest().lat;
      const min_lon = bounds.getSouthWest().lng;
      const max_lat = bounds.getNorthEast().lat;
      const max_lon = bounds.getNorthEast().lng;

      const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";

      const response = await fetch(`${apiUrl}/api/custom_map`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ min_lat, min_lon, max_lat, max_lon }),
      });

      if (!response.ok) {
        const errorText = await response.text();
        setError(errorText || "Failed to create custom map simulation");
        setLoading(false);
        return;
      }

      const data = await response.json();
      sessionStorage.setItem('sim_token', data.token);
      sessionStorage.removeItem('map_name');
      sessionStorage.removeItem('map_saved');
      sessionStorage.removeItem('map_file_uuid');
      router.push(`/map/${data.uuid}`);
    } catch (err: unknown) {
      console.error(err);
      setError(err instanceof Error ? err.message : "Erreur inconnue");
      setLoading(false);
    }
  };

  return (
    <div className="relative w-full h-[600px] bg-slate-100 rounded-lg overflow-hidden border border-gray-300">
      <MapContainer
        center={defaultCenter}
        zoom={defaultZoom}
        style={{ height: '100%', width: '100%' }}
        dragging={false}
        zoomAnimation={false}
        fadeAnimation={false}
        markerZoomAnimation={false}
      >
        <TileLayer
          attribution='&copy; <a href="https://www.openstreetmap.org/">OpenStreetMap</a> contributors'
          url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
        />
        <SelectionEvents isCtrlPressed={isCtrlPressed} onBoundsChange={setBounds} />
        {bounds && (
          <Rectangle bounds={bounds} pathOptions={{ color: 'blue', weight: 2, fillOpacity: 0.2 }} />
        )}
      </MapContainer>

      {/* Overlay UI */}
      <div className="absolute bottom-4 left-1/2 -translate-x-1/2 z-[1000] flex flex-col items-center gap-2">
        {error && (
          <div className="px-4 py-2 bg-red-100 border border-red-400 text-red-700 rounded shadow text-sm max-w-sm text-center">
            {error}
          </div>
        )}
        {bounds ? (
          <button
            onClick={handleImport}
            disabled={loading}
            className="px-6 py-2 bg-blue-600 hover:bg-blue-700 text-white font-semibold rounded shadow-md transition-colors"
          >
            {loading ? 'Importation en cours...' : 'Importer cette zone'}
          </button>
        ) : (
          <div className="px-6 py-2 bg-white/90 rounded shadow text-gray-700 font-medium">
            Cliquez et glissez pour sélectionner une zone
          </div>
        )}
      </div>
    </div>
  );
}
