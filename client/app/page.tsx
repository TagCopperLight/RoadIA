"use client";
import Image from "next/image";
import { useState } from 'react';

export default function Home() {
  const [hovered, setHovered] = useState<number | null>(null);

  return (
    <div>
      <div className="fixed inset-0 bg-white">
        <Image src="/map.png" alt="Map Background" fill preload/>
      </div>
      <div className="flex flex-col h-screen w-screen justify-center items-center z-10">
        <div className="relative z-20 w-[700px] h-[928px] shadow-2xl bg-white/75 rounded-[15px] flex flex-col items-center">
          <div className="pt-[50px]">
            <Image src="/roadia-logo.svg" alt="RoadIA Logo" width={577} height={192}/>
          </div>
        </div>
      </div>
    </div>
  );
}
