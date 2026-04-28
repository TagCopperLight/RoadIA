'use client';

import { ScoreData } from './map/types';

interface ScoreModalProps {
	score: ScoreData;
	onClose: () => void;
}

function RadarChart({ score }: { score: ScoreData }) {
	const size = 300;
	const center = size / 2;
	const radius = (size / 2) * 0.7;

	// Term calculation as in backend
	const successRatio = score.success_rate;
	const timeRatio = score.total_trip_time > 0 ? score.ref_total_trip_time / score.total_trip_time : 0;
	const co2Ratio = score.total_emitted_co2 > 0 ? score.ref_total_emitted_co2 / score.total_emitted_co2 : 0;
	const networkRatio = score.network_length > 0 ? score.ref_network_length / score.network_length : 0;

	const axes = [
		{ label: 'Succès', value: successRatio },
		{ label: 'Temps', value: timeRatio },
		{ label: 'CO2', value: co2Ratio },
		{ label: 'Réseau', value: networkRatio },
	];

	const getPoint = (index: number, value: number) => {
		const angle = (index * (2 * Math.PI)) / axes.length - Math.PI / 2;
		const r = Math.max(0.1, Math.min(1, value)) * radius;
		return {
			x: center + r * Math.cos(angle),
			y: center + r * Math.sin(angle),
		};
	};

	const getLabelPoint = (index: number) => {
		const angle = (index * (2 * Math.PI)) / axes.length - Math.PI / 2;
		const r = radius + 25;
		return {
			x: center + r * Math.cos(angle),
			y: center + r * Math.sin(angle),
		};
	};

	const points = axes.map((axis, i) => {
		const p = getPoint(i, axis.value);
		return `${p.x},${p.y}`;
	}).join(' ');

	return (
		<div className="flex flex-col items-center">
			<svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} className="overflow-visible">
				{/* Background Grid Circles */}
				{[0.25, 0.5, 0.75, 1].map((r) => (
					<circle
						key={r}
						cx={center}
						cy={center}
						r={r * radius}
						fill="none"
						stroke="#e5e7eb"
						strokeWidth="1"
					/>
				))}

				{/* Axes */}
				{axes.map((_, i) => {
					const p = getPoint(i, 1);
					return (
						<line
							key={i}
							x1={center}
							y1={center}
							x2={p.x}
							y2={p.y}
							stroke="#e5e7eb"
							strokeWidth="1"
						/>
					);
				})}

				{/* Labels */}
				{axes.map((axis, i) => {
					const p = getLabelPoint(i);
					return (
						<text
							key={i}
							x={p.x}
							y={p.y}
							textAnchor="middle"
							alignmentBaseline="middle"
							className="text-xs font-bold fill-gray-500 uppercase"
						>
							{axis.label}
						</text>
					);
				})}

				{/* Data Polygon */}
				<polygon
					points={points}
					fill="rgba(59, 130, 246, 0.2)"
					stroke="#3b82f6"
					strokeWidth="2"
					className="drop-shadow-sm transition-all duration-1000"
				/>

				{/* Data Points */}
				{axes.map((axis, i) => {
					const p = getPoint(i, axis.value);
					return (
						<circle
							key={i}
							cx={p.x}
							cy={p.y}
							r="4"
							fill="#3b82f6"
							className="transition-all duration-1000"
						/>
					);
				})}
			</svg>
		</div>
	);
}

export default function ScoreModal({ score, onClose }: ScoreModalProps) {
	return (
		<div className="absolute inset-0 flex items-center justify-center bg-black/40 backdrop-blur-sm z-50">
			<div className="bg-white p-12 rounded-3xl shadow-2xl max-w-5xl w-full mx-4 transform transition-all animate-in fade-in zoom-in duration-300">
				<div className="flex justify-between items-start mb-8">
					<h2 className="text-4xl font-bold text-gray-800">Simulation Terminée</h2>
					<button
						onClick={onClose}
						className="text-gray-400 hover:text-gray-600 transition-colors cursor-pointer"
					>
						<svg className="w-8 h-8" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</button>
				</div>

				<div className="flex flex-col lg:flex-row gap-12 items-center lg:items-start">
					{/* Left: Radar Chart */}
					<div className="flex-1 w-full max-w-md bg-gray-50 p-8 rounded-3xl flex items-center justify-center">
						<RadarChart score={score} />
					</div>

					{/* Right: Stats Details */}
					<div className="flex-1 space-y-6 w-full">
						<div className="bg-gray-50 p-6 rounded-2xl flex justify-between items-center">
							<span className="font-semibold text-2xl">Score Final</span>
							<span className="text-5xl font-black text-gray-900">{score.score.toFixed(1)}/100</span>
						</div>

						<div className="grid grid-cols-2 gap-6">
							<div className="bg-gray-50 p-5 rounded-xl">
								<p className="text-sm text-gray-500 uppercase font-bold mb-2">Taux de réussite</p>
								<p className="text-3xl font-bold text-gray-800">{(score.success_rate * 100).toFixed(0)}%</p>
							</div>
							<div className="bg-gray-50 p-5 rounded-xl">
								<p className="text-sm text-gray-500 uppercase font-bold mb-2">CO2 Émis</p>
								<p className="text-3xl font-bold text-gray-800">
									{score.total_emitted_co2.toFixed(2)}kg
									<span className="block text-sm text-gray-400 font-normal mt-1">Optimal: {score.ref_total_emitted_co2.toFixed(2)}kg</span>
								</p>
							</div>
							<div className="bg-gray-50 p-5 rounded-xl">
								<p className="text-sm text-gray-500 uppercase font-bold mb-2">Temps total</p>
								<p className="text-3xl font-bold text-gray-800">
									{(score.total_trip_time/60).toFixed(0)}min
									<span className="block text-sm text-gray-400 font-normal mt-1">Optimal: {(score.ref_total_trip_time/60).toFixed(0)}min</span>
								</p>
							</div>
							<div className="bg-gray-50 p-5 rounded-xl">
								<p className="text-sm text-gray-500 uppercase font-bold mb-2">Taille du réseau</p>
								<p className="text-3xl font-bold text-gray-800">
									{(score.network_length/1000).toFixed(2)}km
									<span className="block text-sm text-gray-400 font-normal mt-1">Optimal: {(score.ref_network_length /1000).toFixed(2)}km</span>
								</p>
							</div>
						</div>
					</div>
				</div>

				<button
					onClick={onClose}
					className="mt-10 w-full bg-black hover:bg-neutral-800 text-white font-bold py-4 text-xl rounded-2xl transition-colors cursor-pointer"
				>
					Fermer
				</button>
			</div>
		</div>
	);
}