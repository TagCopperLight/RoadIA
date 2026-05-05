'use client';

import dynamic from 'next/dynamic';
import Image from 'next/image';
import Link from 'next/link';

// Dynamically load the MapSelection component with SSR disabled
// Leaflet uses window/document objects which break during Next.js SSR phase
const MapSelection = dynamic(() => import('@/components/MapSelection'), {
  ssr: false,
  loading: () => (
    <div className="w-full h-[600px] bg-slate-100 flex items-center justify-center rounded-lg border border-gray-300">
      <div className="text-gray-500 font-medium text-lg">Chargement de la carte...</div>
    </div>
  ),
});

export default function MapSelectPage() {
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
            <Image src="/home/roadia-logo.svg" alt="RoadIA Logo" width={200} height={66} loading="eager" />
          </div>
        </div>

        {/* Content */}
        <div className="bg-white p-6 shadow-xl rounded-xl">
          <h1 className="text-2xl font-bold text-gray-800 mb-2">Importer une nouvelle carte</h1>
          <p className="text-gray-600 mb-6 font-medium">
            Cliquez et faites glisser avec la souris pour dessiner un rectangle de sélection.<br/>
            Maintenez la touche <kbd className="bg-gray-200 px-1 rounded">Ctrl</kbd> enfoncée et cliquez pour déplacer la carte.
          </p>

          {/* Setup dynamic leaflet component */}
          <MapSelection />
        </div>
      </div>
    </div>
  );
}
