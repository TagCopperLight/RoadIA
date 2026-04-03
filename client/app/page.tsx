'use client';

import Image from "next/image";
import { useRouter } from "next/navigation";
import { createSimulation } from "@/app/websocket/websocket";

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

  const handleNewSimulation = async () => {
    const { uuid, token } = await createSimulation();
    sessionStorage.setItem('sim_token', token);
    router.push(`/map/${uuid}`);
  };

  return (
    <div>
      <div className="fixed inset-0 bg-white">
        <Image src="/home/map.png" alt="Map Background" fill />
      </div>
      <div className="flex flex-col h-screen w-screen justify-center items-center z-10">
        <div className="relative z-20 w-[700px] h-[928px] shadow-2xl bg-white/75 rounded-[15px] flex flex-col items-center">
          <div className="pt-[50px]">
            <Image src="/home/roadia-logo.svg" alt="RoadIA Logo" width={577} height={192} loading="eager" />
          </div>
          <div className="flex pt-[80px]">
            <MenuCard src="/home/new.svg" alt="New" label="Nouveau" onClick={handleNewSimulation} />
            <MenuCard src="/home/folder.svg" alt="Folder" label="Cartes" className="ml-[80px]" />
          </div>
          <MenuCard src="/home/trophy.svg" alt="Trophy" label="Challenges" className="mt-[80px]" />
          <div className="mt-auto pb-[16px]">
            <Image src="/home/bagnole-logo.png" alt="Bagnole Logo" width={190} height={47} loading="eager" />
          </div>
        </div>
      </div>
    </div>
  );
}
