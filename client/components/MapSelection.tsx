'use client';

import { useState, useEffect } from 'react';
import { MapContainer, TileLayer, useMapEvents, Rectangle } from 'react-leaflet';
import 'leaflet/dist/leaflet.css';
import { LatLngExpression, LatLngBoundsExpression, LatLngBounds } from 'leaflet';
import { useRouter } from 'next/navigation';

export default function MapSelection() {
  const router = useRouter();
  const [bounds, setBounds] = useState<LatLngBounds | null>(null);
  const [isSelecting, setIsSelecting] = useState(false);
  const [startPoint, setStartPoint] = useState<LatLngExpression | null>(null);
  const [endPoint, setEndPoint] = useState<LatLngExpression | null>(null);
  const [loading, setLoading] = useState(false);
  const [isCtrlPressed, setIsCtrlPressed] = useState(false);

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

  // Set default center to Lannion, France (or you could use geolocation)
  const defaultCenter: LatLngExpression = [48.7323, -3.4589];
  const defaultZoom = 13;

  function SelectionEvents() {
    const map = useMapEvents({
      mousedown(e) {
        if (isCtrlPressed || e.originalEvent.ctrlKey) return; // Do not select if Ctrl is pressed

        setIsSelecting(true);
        setStartPoint(e.latlng);
        setEndPoint(e.latlng);
        setBounds(null); // Clear previous selection
      },
      mousemove(e) {
        if (!isSelecting) return;
        setEndPoint(e.latlng);
      },
      mouseup(e) {
        if (!isSelecting) return;
        setIsSelecting(false);
        if (startPoint && endPoint) {
          // Leaflet's LatLngBounds can be derived from two points
          // eslint-disable-next-line @typescript-eslint/no-explicit-any
          const L = (window as any).L; // quick bypass for importing leafet safely in NextJS (SSR issues)
          if (L) {
             const newBounds = L.latLngBounds(startPoint, e.latlng);
             setBounds(newBounds);
          }
        }
      },
    });

    useEffect(() => {
      if (isCtrlPressed) {
        map.dragging.enable();
      } else {
        map.dragging.disable();
      }
    }, [map]);

    return null;
  }

  // Workaround to ensure Leaflet library works without SSR issues
  useEffect(() => {
    // dynamically importing leaflet inside effect if necessary, but usually react-leaflet handles it
    // if using next dynamic, we can just load the component dynamically
  }, []);

  const handleImport = async () => {
    if (!bounds) return;

    // Check size limit: Overpass API can timeout or fail if area is too huge.
    // Calculate diagonal distance in meters.
    const distanceMeters = bounds.getSouthWest().distanceTo(bounds.getNorthEast());
    const MAX_DIAGONAL_METERS = 10000; // 10 km
    if (distanceMeters > MAX_DIAGONAL_METERS) {
       alert(`La zone sélectionnée est trop grande (${(distanceMeters/1000).toFixed(1)} km de diagonale). Veuillez sélectionner une zone plus petite (diagonale max : ${MAX_DIAGONAL_METERS/1000} km).`);
       return;
    }

    setLoading(true);
    try {
      // Get min and max coordinates
      const min_lat = bounds.getSouthWest().lat;
      const min_lon = bounds.getSouthWest().lng;
      const max_lat = bounds.getNorthEast().lat;
      const max_lon = bounds.getNorthEast().lng;

      const apiUrl = process.env.NEXT_PUBLIC_API_URL || "http://localhost:8080";
      
      const response = await fetch(`${apiUrl}/api/custom_map`, {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          min_lat,
          min_lon,
          max_lat,
          max_lon,
        }),
      });

      if (!response.ok) {
        const errorText = await response.text();
        throw new Error(errorText || "Failed to create custom map simulation");
      }

      const data = await response.json();
      sessionStorage.setItem('sim_token', data.token);
      router.push(`/map/${data.uuid}`);
    } catch (err: unknown) {
      console.error(err);
      const errorMessage = err instanceof Error ? err.message : "Erreur inconnue";
      alert(`Erreur lors de l'import de la carte : ${errorMessage}`);
      setLoading(false);
    }
  };

  const getRectangleBounds = (): LatLngBoundsExpression | null => {
    if (bounds) return bounds;
    if (isSelecting && startPoint && endPoint) {
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      const L = (window as any).L;
      if (L) return L.latLngBounds(startPoint, endPoint);
    }
    return null;
  };

  const rectBounds = getRectangleBounds();

  return (
    <div className="relative w-full h-[600px] bg-slate-100 rounded-lg overflow-hidden border border-gray-300">
      <MapContainer
        center={defaultCenter}
        zoom={defaultZoom}
        style={{ height: '100%', width: '100%' }}
        dragging={false} // Initially disabled, dynamically managed via SelectionEvents
      >
        <TileLayer
          attribution='&copy; <a href="https://www.openstreetmap.org/">OpenStreetMap</a> contributors'
          url="https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png"
        />
        <SelectionEvents />
        {rectBounds && (
          <Rectangle bounds={rectBounds} pathOptions={{ color: 'blue', weight: 2, fillOpacity: 0.2 }} />
        )}
      </MapContainer>

      {/* Overlay UI */}
      <div className="absolute bottom-4 left-1/2 -translate-x-1/2 z-[1000] flex flex-col items-center gap-2">
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
