'use client';

import { useState } from 'react';

interface LegendItem {
    label: string;
    visual: React.ReactNode;
}

function ColorDot({ color }: { color: string }) {
    return <span className="inline-block w-3 h-3 rounded-full flex-shrink-0" style={{ backgroundColor: color }} />;
}

function OneWayRoad() {
    return (
        <svg width="32" height="12" viewBox="0 0 32 12">
            <rect x="0" y="1" width="32" height="10" fill="#555555" />
            <line x1="0" y1="1" x2="32" y2="1" stroke="white" strokeWidth="1" />
            <line x1="0" y1="11" x2="32" y2="11" stroke="white" strokeWidth="1" />
            {/* left edge (one-way marker) */}
            <line x1="0" y1="1" x2="0" y2="11" stroke="white" strokeWidth="1" />
            {/* direction arrow */}
            <polygon points="22,6 16,3.5 16,8.5" fill="#888888" />
        </svg>
    );
}

function TwoWayRoad() {
    return (
        <svg width="32" height="14" viewBox="0 0 32 14">
            <rect x="0" y="0" width="32" height="14" fill="#555555" />
            <line x1="0" y1="0" x2="32" y2="0" stroke="white" strokeWidth="1" />
            <line x1="0" y1="14" x2="32" y2="14" stroke="white" strokeWidth="1" />
            {/* double yellow center lines */}
            <line x1="0" y1="6" x2="32" y2="6" stroke="#f59e0b" strokeWidth="1" />
            <line x1="0" y1="8" x2="32" y2="8" stroke="#f59e0b" strokeWidth="1" />
        </svg>
    );
}

const LEGEND_ITEMS: LegendItem[] = [
    { label: 'Intersection', visual: <ColorDot color="#555555" /> },
    { label: 'Habitation', visual: <ColorDot color="#3b82f6" /> },
    { label: 'Travail', visual: <ColorDot color="#ef4444" /> },
    { label: 'Route bidirectionnelle', visual: <TwoWayRoad /> },
    { label: 'Route à sens unique', visual: <OneWayRoad /> },
    { label: 'Hybride', visual: <ColorDot color="#a855f7" /> },
    { label: 'Électrique', visual: <ColorDot color="#06b6d4" /> },
    { label: 'Essence', visual: <ColorDot color="#f59e0b" /> },
    { label: 'Diesel', visual: <ColorDot color="#8b7355" /> },
];

export default function Legend() {
    const [open, setOpen] = useState(false);

    return (
        <div className="absolute top-[10px] right-[10px] flex flex-col items-end gap-2 z-30">
            <button
                onClick={() => setOpen(v => !v)}
                title={open ? 'Fermer la légende' : 'Afficher la légende'}
                className="cursor-pointer w-7 h-7 rounded-full bg-neutral-900/90 text-white text-sm font-bold shadow-md hover:bg-neutral-700 flex items-center justify-center"
            >
                ?
            </button>
            {open && (
                <div className="bg-neutral-900/95 text-white rounded-xl shadow-xl p-4 flex flex-col gap-2 min-w-[190px]">
                    <p className="text-xs font-semibold text-neutral-400 uppercase tracking-wide mb-1">Légende</p>
                    {LEGEND_ITEMS.map(({ label, visual }) => (
                        <div key={label} className="flex items-center gap-3 text-sm">
                            <div className="w-8 flex items-center justify-center flex-shrink-0">{visual}</div>
                            <span className="text-neutral-200">{label}</span>
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
