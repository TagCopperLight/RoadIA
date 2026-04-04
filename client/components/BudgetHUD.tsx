'use client';

import { MapData } from './map/types';
import { calculateCost, MAX_BUDGET } from './map/budget';

interface BudgetHUDProps {
    mapData: MapData | null;
}

function fmt(n: number): string {
    if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(2)}M`;
    if (n >= 1_000)     return `${(n / 1_000).toFixed(1)}k`;
    return n.toFixed(0);
}

export default function BudgetHUD({ mapData }: BudgetHUDProps) {
    const spent = mapData ? calculateCost(mapData) : 0;
    const ratio = Math.min(spent / MAX_BUDGET, 1);
    const barColor =
        ratio > 0.9 ? 'bg-red-500' :
        ratio > 0.7 ? 'bg-amber-400' :
        'bg-green-400';

    return (
        <div className="absolute bottom-[15px] left-[15px] bg-black/80 text-white rounded-[10px] shadow-md px-3 py-2 min-w-[160px]">
            <p className="text-xs text-gray-400 uppercase tracking-wide mb-1">Budget</p>
            <div className="w-full h-1.5 bg-gray-700 rounded-full mb-1.5 overflow-hidden">
                <div
                    className={`h-full rounded-full transition-all duration-300 ${barColor}`}
                    style={{ width: `${Math.round(ratio * 100)}%` }}
                />
            </div>
            <div className="flex justify-between items-baseline">
                <span className="text-sm font-semibold tabular-nums">{fmt(spent)}</span>
                <span className="text-xs text-gray-400">/ {fmt(MAX_BUDGET)}</span>
            </div>
        </div>
    );
}
