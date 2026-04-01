import Image from 'next/image';
import MapShell from '@/components/MapShell';

const MENU_ITEMS = ['Fichier', 'Édition', 'Simulation', 'Paramètres', 'Statistiques'];

function Header() {
    return (
        <div className="flex items-center w-full p-[15px]">
            <Image src="/map/logo-black.svg" alt="Logo" width={45} height={45} loading='eager' />
            <div className='flex flex-col pl-[15px]'>
                <p className='text-[17px]'>Lannion - 2025</p>
                <div className='flex text-[15px] font-medium'>
                    {MENU_ITEMS.map((item) => (
                        <p key={item} className='mr-[14px] cursor-pointer hover:opacity-50 transition-opacity select-none'>
                            {item}
                        </p>
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


export default async function MapPage({ params }: { params: Promise<{ uuid: string }> }) {
  const { uuid } = await params;
  return (
    <div className="flex flex-col h-screen w-screen items-center">
        <Header />
        <MapShell uuid={uuid} />
    </div>
  );
}
