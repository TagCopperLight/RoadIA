'use client';

import { ScoreData } from './map/types';

interface ScoreModalProps {
	score: ScoreData;
	onClose: () => void;
}

export default function ScoreModal({ score, onClose }: ScoreModalProps) {
	return (
		<div className="absolute inset-0 flex items-center justify-center bg-black/40 backdrop-blur-sm z-50">
			<div className="bg-white p-8 rounded-2xl shadow-2xl max-w-md w-full mx-4 transform transition-all animate-in fade-in zoom-in duration-300">
				<div className="flex justify-between items-start mb-6">
					<h2 className="text-3xl font-bold text-gray-800">Simulation Terminée</h2>
					<button
						onClick={onClose}
						className="text-gray-400 hover:text-gray-600 transition-colors cursor-pointer"
					>
						<svg className="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
							<path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M6 18L18 6M6 6l12 12" />
						</svg>
					</button>
				</div>

				<div className="space-y-4">
					<div className="bg-gray-50 p-4 rounded-xl flex justify-between items-center">
						<span className="font-semibold text-lg">Score Final</span>
						<span className="text-3xl font-black text-gray-900">{score.score.toFixed(3)}</span>
					</div>

					<div className="grid grid-cols-2 gap-4">
						<div className="bg-gray-50 p-3 rounded-lg">
							<p className="text-xs text-gray-500 uppercase font-bold mb-1">Taux de réussite</p>
							<p className="text-xl font-bold text-gray-800">{(score.success_rate * 100).toFixed(0)}%</p>
						</div>
						<div className="bg-gray-50 p-3 rounded-lg">
							<p className="text-xs text-gray-500 uppercase font-bold mb-1">CO2 Émis</p>
							<p className="text-xl font-bold text-gray-800">{score.total_emitted_co2.toFixed(2)}kg</p>
						</div>
						<div className="bg-gray-50 p-3 rounded-lg">
							<p className="text-xs text-gray-500 uppercase font-bold mb-1">Temps total</p>
							<p className="text-xl font-bold text-gray-800">{score.total_trip_time.toFixed(0)}s</p>
						</div>
						<div className="bg-gray-50 p-3 rounded-lg">
							<p className="text-xs text-gray-500 uppercase font-bold mb-1">Distance parcourue</p>
							<p className="text-xl font-bold text-gray-800">{(score.total_distance_traveled / 1000).toFixed(2)}km</p>
						</div>
					</div>
				</div>

				<button
					onClick={onClose}
					className="mt-8 w-full bg-black hover:bg-neutral-800 text-white font-bold py-3 rounded-xl transition-colors cursor-pointer"
				>
					Fermer
				</button>
			</div>
		</div>
	);
}