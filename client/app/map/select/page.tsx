'use client';

import dynamic from 'next/dynamic';
import Image from 'next/image';
import Link from 'next/link';
import { useState } from 'react';
import { useRouter } from 'next/navigation';

const MapSelection = dynamic(() => import('@/components/MapSelection'), {
  ssr: false,
  loading: () => (
    <div className="w-full h-[600px] bg-slate-100 flex items-center justify-center rounded-lg border border-gray-300">
      <div className="text-gray-500 font-medium text-lg">Chargement de la carte...</div>
    </div>
  ),
});

export default function MapSelectPage() {
  const router = useRouter();
  const [voidLoading, setVoidLoading] = useState(false);
  const [voidError, setVoidError] = useState<string | null>(null);

  const handleCreateVoidMap = async () => {
    setVoidError(null);
    setVoidLoading(true);
    try {
      const apiUrl = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';
      const response = await fetch(`${apiUrl}/api/simulations`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: '{}',
      });
      if (!response.ok) {
        setVoidError('Erreur lors de la création de la carte vide.');
        setVoidLoading(false);
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
      setVoidError(err instanceof Error ? err.message : 'Erreur inconnue.');
      setVoidLoading(false);
    }
  };

  return (
    <div className="min-h-screen bg-slate-50 flex flex-col items-center py-10 px-4">
      <div className="max-w-4xl w-full">
        {/* Header / Top bar */}
        <div className="flex items-center mb-6">
          <Link
            href="/"
            className="flex items-center text-gray-600 hover:text-gray-900 transition-colors"
          >
            <span className="text-2xl mr-2">←</span> Retour
          </Link>
          <div className="mx-auto flex gap-4">
            <Image src="/home/roadia-logo.svg" alt="RoadIA Logo" width={200} height={67} loading="eager" />
          </div>
        </div>

        <div className="bg-white p-6 shadow-xl rounded-xl mb-6">
          <h1 className="text-2xl font-bold text-gray-800 mb-2">Importer une nouvelle carte</h1>
          <p className="text-gray-600 mb-6 font-medium">
            Cliquez et faites glisser avec la souris pour dessiner un rectangle de sélection.<br/>
            Maintenez la touche <kbd className="bg-gray-200 px-1 rounded">Ctrl</kbd> enfoncée et cliquez pour déplacer la carte.
          </p>

          <MapSelection />
        </div>

        <div className="bg-white p-6 shadow-xl rounded-xl">
          <h2 className="text-xl font-bold text-gray-800 mb-2">Créer une carte vide</h2>
          <p className="text-gray-600 mb-4 font-medium">
            Démarrez avec une carte sans routes et construisez votre réseau routier depuis zéro.
          </p>
          {voidError && (
            <div className="mb-4 px-4 py-2 bg-red-100 border border-red-400 text-red-700 rounded text-sm">
              {voidError}
            </div>
          )}
          <button
            onClick={handleCreateVoidMap}
            disabled={voidLoading}
            className="px-6 py-2 bg-gray-700 hover:bg-gray-800 text-white font-semibold rounded shadow-md transition-colors disabled:opacity-60"
          >
            {voidLoading ? 'Création en cours...' : 'Créer une carte vide'}
          </button>
        </div>
      </div>
    </div>
  );
}
