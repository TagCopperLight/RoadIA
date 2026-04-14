'use client';

import { ScoreData } from './map/types';

interface ScoreModalProps {
	score: ScoreData;
	onClose: () => void;
}

export default function ScoreModal({ score, onClose }: ScoreModalProps) {
	return (
		<div className="absolute inset-0 flex items-center justify-center bg-black/40 backdrop-blur-sm z-50">
			<div className="bg-white p-12 rounded-3xl shadow-2xl max-w-2xl w-full mx-4 transform transition-all animate-in fade-in zoom-in duration-300">
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

				<div className="space-y-6">
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