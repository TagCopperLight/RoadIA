'use client';

import { useState } from 'react';

interface LegendItem {
    label: string;
    color?: string;
    type?: 'node' | 'road-twoway' | 'road-oneway';
    subtext?: string;
}

export default function Legend() {
    const [isExpanded, setIsExpanded] = useState(false);

    const legendItems: LegendItem[] = [
        { label: 'Intersection', color: '#888888', type: 'node' },
        { label: 'Habitation', color: '#3b82f6', type: 'node' },
        { label: 'Workplace', color: '#ef4444', type: 'node' },
        { label: 'Route bidirectionnelle', type: 'road-twoway' },
        { label: 'Route unidirectionnelle', type: 'road-oneway' },
    ];

    const renderLegendShape = (item: LegendItem) => {
        switch (item.type) {
            case 'node':
                return (
                    <div
                        className="w-5 h-5 rounded-full flex-shrink-0"
                        style={{ backgroundColor: item.color, border: `2px solid ${item.color}` }}
                    />
                );
            case 'road-twoway':
                return (
                    <div className="w-8 h-4 flex-shrink-0 relative flex items-center">
                        {/* Road background */}
                        <div className="absolute inset-0 bg-gray-600 rounded-sm" />
                        {/* Double yellow center line */}
                        <div className="absolute left-0 right-0 top-1/2 transform -translate-y-1/2">
                            <div className="h-0.5 bg-yellow-500" style={{ marginBottom: '1px' }} />
                            <div className="h-0.5 bg-yellow-500" />
                        </div>
                        {/* White edges */}
                        <div className="absolute left-0 right-0 top-0 border-t border-white" />
                        <div className="absolute left-0 right-0 bottom-0 border-b border-white" />
                    </div>
                );
            case 'road-oneway':
                return (
                    <div className="w-8 h-4 flex-shrink-0 relative flex items-center justify-center">
                        {/* Road background */}
                        <div className="absolute inset-0 bg-gray-600 rounded-sm" />
                        {/* White center line */}
                        <div className="absolute left-0 right-0 top-1/2 h-0.5 bg-white transform -translate-y-1/2" />
                        {/* Direction arrow */}
                        <div className="absolute right-1 text-gray-400 text-xs">▶</div>
                        {/* White edges */}
                        <div className="absolute left-0 right-0 top-0 border-t border-white" />
                        <div className="absolute left-0 right-0 bottom-0 border-b border-white" />
                    </div>
                );
            default:
                return null;
        }
    };

    return (
        <div className="fixed bottom-[calc(20px+60px)] right-[15px] bg-black rounded-[10px] border border-gray-700 text-white z-40" style={{ maxWidth: '300px' }}>
            {/* Header - Always visible */}
            <button
                onClick={() => setIsExpanded(!isExpanded)}
                className="w-full flex items-center justify-between p-3 hover:bg-gray-900 transition-colors cursor-pointer"
            >
                <span className="text-sm font-semibold">Légende</span>
                <span className="text-lg transition-transform" style={{ transform: isExpanded ? 'rotate(180deg)' : 'rotate(0deg)' }}>
                    ▼
                </span>
            </button>

            {/* Content - Toggleable */}
            {isExpanded && (
                <div className="border-t border-gray-700 p-3 flex flex-col gap-3">
                    {legendItems.map((item, index) => (
                        <div key={index}>
                            <div className="flex items-center gap-2.5">
                                {renderLegendShape(item)}
                                <span className="text-xs text-gray-300 font-medium">{item.label}</span>
                            </div>
                            {item.subtext && (
                                <div className="ml-7 text-xs text-gray-500 mt-1">
                                    {item.subtext}
                                </div>
                            )}
                        </div>
                    ))}
                </div>
            )}
        </div>
    );
}
