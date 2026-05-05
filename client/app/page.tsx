'use client';

import Image from "next/image";
import { useRouter } from "next/navigation";
import { listMaps, loadMap } from "@/app/websocket/websocket";
import { useState } from "react";

interface MenuCardProps {
  src: string;
  alt: string;
  label: string;
  className?: string;
  onClick?: () => void;
}

function MenuCard({ src, alt, label, className = "", onClick }: MenuCardProps) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={`flex flex-col items-center justify-center w-[200px] h-[200px] bg-black hover:bg-zinc-800 rounded-[10px] drop-shadow-[6px_6px_5px_rgba(0,0,0,0.35)] cursor-pointer ${className}`}
    >
      <Image src={src} alt={alt} width={130} height={130} loading="eager" />
      <p className="text-[22px] text-white text-center pt-[8px]">{label}</p>
    </button>
  );
}

export default function Home() {
  const router = useRouter();
  const [showMapList, setShowMapList] = useState(false);
  const [maps, setMaps] = useState<string[]>([]);

  const handleNewSimulation = () => {
    router.push('/map/select');
  };

  const handleShowMaps = async () => {
    try {
      const result = await listMaps();
      setMaps(result.maps);
      setShowMapList(true);
    } catch (e) {
      console.error("Failed to list maps:", e);
    }
  };

  const handleLoadMap = async (filename: string) => {
    try {
      const { uuid, token } = await loadMap(filename);
      sessionStorage.setItem('sim_token', token);
      sessionStorage.setItem('map_name', filename.replace(/\.json$/, ''));
      sessionStorage.setItem('map_saved', 'true');
      router.push(`/map/${uuid}`);
    } catch (e) {
      console.error("Failed to load map:", e);
    }
  };

  return (
    <div>
      <div className="fixed inset-0 bg-white">
        <Image src="/home/map.png" alt="Map Background" fill />
      </div>
      <div className="flex flex-col h-screen w-screen justify-center items-center z-10">
        <div className="relative z-20 w-[700px] h-[928px] shadow-2xl bg-white/75 rounded-[15px] flex flex-col items-center overflow-hidden">
          <div className="pt-[50px]">
            <Image src="/home/roadia-logo.svg" alt="RoadIA Logo" width={577} height={192} loading="eager" />
          </div>

          {!showMapList ? (
            <>
              <div className="flex pt-[80px]">
                <MenuCard src="/home/new.svg" alt="New" label="Nouveau" onClick={handleNewSimulation} />
                <MenuCard src="/home/folder.svg" alt="Folder" label="Cartes" className="ml-[80px]" onClick={handleShowMaps} />
              </div>
              <MenuCard src="/home/trophy.svg" alt="Trophy" label="Challenges" className="mt-[80px]" />
            </>
          ) : (
            <div className="flex flex-col items-center w-full px-10 pt-10 overflow-hidden">
              <h2 className="text-2xl font-bold mb-6">Cartes Sauvegardées</h2>
              <div className="flex-1 w-full overflow-y-auto mb-6 bg-white/50 rounded-lg border border-gray-200">
                {maps.length === 0 ? (
                  <p className="text-center py-10 text-gray-500">Aucune carte trouvée dans data/</p>
                ) : (
                  <ul className="divide-y divide-gray-200">
                    {maps.map((map) => (
                      <li key={map} className="flex justify-between items-center p-4 bg-white/80 hover:bg-gray-100">
                        <span className="font-medium">{map.replace(/\.json$/, '')}</span>
                        <button
                          onClick={() => handleLoadMap(map)}
                          className="cursor-pointer bg-black text-white px-4 py-2 rounded hover:bg-zinc-800 transition-colors"
                        >
                          Charger
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
              <button
                onClick={() => setShowMapList(false)}
                className="cursor-pointer mb-10 text-zinc-600 hover:text-black font-semibold"
              >
                ← Retour
              </button>
            </div>
          )}

          <div className="mt-auto pb-[16px]">
            <Image src="/home/bagnole-logo.png" alt="Bagnole Logo" width={190} height={47} loading="eager" />
          </div>
        </div>
      </div>
    </div>
  );
}
