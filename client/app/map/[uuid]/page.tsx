'use client';

import Image from 'next/image';
import MapShell from '@/components/MapShell';
import { useEditMode } from '@/components/EditModeContext';
import { use, useState, useRef, useEffect } from 'react';
import { useRouter, useParams } from 'next/navigation';
import { saveMap, renameMap, deleteMap, useWs } from '@/app/websocket/websocket';

const MENU_ITEMS = ['Fichier', 'Édition', 'Simulation', 'Paramètres', 'Statistiques'];

function Header() {
    const { isScoringLoading, setIsScoringLoading, showScore, setShowSettings } = useEditMode();
    const ws = useWs();
    const router = useRouter();
    const params = useParams();
    const [openMenu, setOpenMenu] = useState<string | null>(null);
    const [mapName, setMapName] = useState<string>(() => {
        if (typeof window === 'undefined') return 'Nouvelle carte';
        return sessionStorage.getItem('map_name') ?? 'Nouvelle carte';
    });
    const [savedName, setSavedName] = useState<string | null>(() => {
        if (typeof window === 'undefined') return null;
        if (sessionStorage.getItem('map_saved') !== 'true') return null;
        return sessionStorage.getItem('map_name');
    });

    const menuRef = useRef<HTMLDivElement>(null);

    useEffect(() => {
        function handleClickOutside(e: MouseEvent) {
            if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
                setOpenMenu(null);
            }
        }
        document.addEventListener('mousedown', handleClickOutside);
        return () => document.removeEventListener('mousedown', handleClickOutside);
    }, []);

    const handleSupprimer = async () => {
        setOpenMenu(null);
        if (!confirm(`Supprimer « ${mapName} » ? Cette action est irréversible.`)) return;
        const fileUuid = sessionStorage.getItem('map_file_uuid');
        if (!fileUuid || sessionStorage.getItem('map_saved') !== 'true') {
            router.push('/');
            return;
        }
        try {
            await deleteMap(fileUuid);
        } catch (e) {
            console.error('Failed to delete map:', e);
        }
        router.push('/');
    };

    const handleSauvegarder = async () => {
        setOpenMenu(null);
        const uuid = params.uuid as string;
        const token = sessionStorage.getItem('sim_token');
        if (!uuid || !token) {
            console.error('Identifiants de simulation manquants');
            return;
        }
        try {
            const existingFileUuid = sessionStorage.getItem('map_file_uuid') ?? undefined;
            const result = await saveMap(uuid, token, mapName, existingFileUuid);
            sessionStorage.setItem('map_file_uuid', result.file_uuid);
            sessionStorage.setItem('map_name', mapName);
            sessionStorage.setItem('map_saved', 'true');
            setSavedName(mapName);
        } catch (e) {
            console.error('Failed to save map:', e);
        }
    };

    const handleNameBlur = async () => {
        if (savedName === null || mapName === savedName) return;
        const fileUuid = sessionStorage.getItem('map_file_uuid');
        if (!fileUuid) return;
        try {
            await renameMap(fileUuid, mapName);
            sessionStorage.setItem('map_name', mapName);
            setSavedName(mapName);
        } catch (e) {
            console.error('Failed to rename map:', e);
        }
    };

    return (
        <div className="flex items-center w-full p-[15px]">
            <button onClick={() => router.push('/')} className="cursor-pointer hover:opacity-70 transition-opacity">
                <Image src="/map/logo-black.svg" alt="Logo" width={45} height={45} loading='eager' />
            </button>
            <div className='flex flex-col pl-[15px]'>
                <div className='inline-grid w-fit text-[17px]'>
                    <span suppressHydrationWarning className='invisible whitespace-pre col-start-1 row-start-1 pl-1 pr-2 pointer-events-none'>{mapName || ' '}</span>
                    <input
                        suppressHydrationWarning
                        value={mapName}
                        size={1}
                        onChange={(e) => {
                            setMapName(e.target.value);
                            sessionStorage.setItem('map_name', e.target.value);
                        }}
                        onBlur={handleNameBlur}
                        className='col-start-1 row-start-1 min-w-0 bg-transparent border border-transparent rounded px-1 outline-none hover:border-gray-300 focus:border-gray-400 transition-colors'
                    />
                </div>
                <div className='flex text-[15px] font-medium pl-1' ref={menuRef}>
                    {MENU_ITEMS.map((item) => (
                        <div key={item} className='relative mr-[14px]'>
                            <p
                                onClick={() => {
                                    if (item === 'Statistiques') { if (!isScoringLoading && !showScore) { ws?.send('requestScore', {}); setIsScoringLoading(true); } return; }
                                    if (item === 'Fichier') { setOpenMenu(openMenu === 'Fichier' ? null : 'Fichier'); return; }
                                    if (item === 'Paramètres') { setOpenMenu(null); setShowSettings(true); return; }
                                }}
                                className={`transition-opacity select-none ${item === 'Statistiques' && (isScoringLoading || showScore) ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer hover:opacity-50'}`}
                            >
                                {item}
                            </p>
                            {item === 'Fichier' && openMenu === 'Fichier' && (
                                <div className='absolute top-full left-0 mt-1 bg-white border border-gray-200 rounded shadow-md z-50 min-w-[160px]'>
                                    <p
                                        onClick={handleSauvegarder}
                                        className='px-4 py-2 text-[14px] cursor-pointer hover:bg-gray-100 select-none'
                                    >
                                        Sauvegarder
                                    </p>
                                    <p
                                        onClick={handleSupprimer}
                                        className='px-4 py-2 text-[14px] cursor-pointer hover:bg-red-50 text-red-600 select-none'
                                    >
                                        Supprimer
                                    </p>
                                </div>
                            )}
                        </div>
                    ))}
                </div>
            </div>
            <button className='ml-auto cursor-pointer bg-black hover:bg-neutral-800 transition-colors rounded-[10px] flex items-center justify-center'>
                <Image src="/map/send.svg" alt="Share" width={20} height={20} className='m-[11px]' />
                <p className='text-white text-[17px] font-medium mr-[11px]'>Partager</p>
            </button>
        </div>
    )
}


export default function MapPage({ params }: { params: Promise<{ uuid: string }> }) {
  const { uuid } = use(params);
  return (
    <EditModeProviderWrapper uuid={uuid} />
  );
}

function EditModeProviderWrapper({ uuid }: { uuid: string }) {
    return (
        <div className="flex flex-col h-screen w-screen items-center">
            <MapShell uuid={uuid}>
                <Header />
            </MapShell>
        </div>
    )
}
