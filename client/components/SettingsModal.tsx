'use client';

import { useState, useEffect } from 'react';
import { MapSettings, useEditMode } from './EditModeContext';
import { useWs } from '@/app/websocket/websocket';

interface SettingsModalProps {
    uuid: string;
    onClose: () => void;
}

type Section = 'simulation' | 'budget' | 'score';

const API_URL = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080';
const SIMULATION_START_TIME_MAX_S = 39_600;
const SIMULATION_DURATION_MAX_S = 86_400;
const TIME_STEP_MIN_S = 0.01;
const TIME_STEP_MAX_S = 1;

export default function SettingsModal({ uuid, onClose }: SettingsModalProps) {
    const { setMapSettings, setSimState, setShowScore, setSimulationResetAt } = useEditMode();
    const ws = useWs();
    const [form, setForm] = useState<MapSettings | null>(null);
    const [loading, setLoading] = useState(true);
    const [saving, setSaving] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [section, setSection] = useState<Section>('simulation');

    useEffect(() => {
        fetch(`${API_URL}/api/simulations/${uuid}/settings`)
            .then(r => r.json())
            .then((data: MapSettings) => {
                setForm(data);
                setMapSettings(data);
            })
            .catch(() => setError('Impossible de charger les paramètres.'))
            .finally(() => setLoading(false));
    }, [uuid, setMapSettings]);

    const set = <K extends keyof MapSettings>(key: K, value: MapSettings[K]) => {
        setForm(prev => prev ? { ...prev, [key]: value } : prev);
    };

    const weightSum = form
        ? +(form.time_weight + form.success_weight + form.pollution_weight + form.infrastructure_weight).toFixed(4)
        : 0;
    const weightsValid = Math.abs(weightSum - 1) <= 0.01;

    const handleSave = async () => {
        if (!form || !weightsValid) return;
        setSaving(true);
        setError(null);
        try {
            const res = await fetch(`${API_URL}/api/simulations/${uuid}/settings`, {
                method: 'POST',
                headers: { 'Content-Type': 'application/json' },
                body: JSON.stringify(form),
            });
            if (!res.ok) {
                const msg = await res.text();
                setError(msg || 'Erreur lors de la sauvegarde.');
                return;
            }
            setMapSettings(form);
            ws?.send('resetSimulation', {});
            setSimState('stopped');
            setShowScore(false);
            setSimulationResetAt(prev => prev + 1);
            onClose();
        } catch {
            setError('Erreur réseau.');
        } finally {
            setSaving(false);
        }
    };

    return (
        <div className="absolute inset-0 flex items-center justify-center bg-black/40 backdrop-blur-sm z-50">
            <div className="bg-white rounded-3xl shadow-2xl w-full max-w-2xl mx-4 flex flex-col max-h-[90vh]">
                {/* Header */}
                <div className="flex items-center justify-between px-8 pt-8 pb-4">
                    <h2 className="text-2xl font-semibold text-gray-900">Paramètres</h2>
                    <button
                        onClick={onClose}
                        className="text-gray-400 hover:text-gray-600 transition-colors text-2xl leading-none"
                    >
                        ×
                    </button>
                </div>

                {/* Tabs */}
                <div className="flex px-4 gap-1 border-b border-gray-100">
                    {(['simulation', 'budget', 'score'] as Section[]).map(s => (
                        <button
                            key={s}
                            onClick={() => setSection(s)}
                            className={`px-4 py-2 text-sm font-medium rounded-t-lg transition-colors capitalize ${
                                section === s
                                    ? 'bg-gray-100 text-gray-900'
                                    : 'text-gray-500 hover:text-gray-700'
                            }`}
                        >
                            {s === 'simulation' ? 'Simulation' : s === 'budget' ? 'Budget & Économie' : 'Score'}
                        </button>
                    ))}
                </div>

                {/* Body */}
                <div className="overflow-y-auto flex-1 px-8 py-6">
                    {loading && <p className="text-gray-500 text-sm">Chargement…</p>}
                    {!loading && !form && <p className="text-red-500 text-sm">{error}</p>}
                    {form && (
                        <>
                            {section === 'simulation' && (
                                <div className="space-y-5">
                                    <FieldRow
                                        label="Nombre de trajets"
                                        hint="Chaque trajet crée 2 véhicules"
                                    >
                                        <NumberInput
                                            value={form.vehicle_count}
                                            min={1} max={5000} step={50}
                                            onChange={v => set('vehicle_count', v)}
                                        />
                                    </FieldRow>
                                    <FieldRow
                                        label="Durée de simulation"
                                        hint="En secondes"
                                    >
                                        <NumberInput
                                            value={form.simulation_duration}
                                            min={60} max={SIMULATION_DURATION_MAX_S} step={60}
                                            onChange={v => set('simulation_duration', v)}
                                        />
                                    </FieldRow>
                                    <FieldRow
                                        label="Heure de début"
                                        hint="En secondes depuis minuit"
                                    >
                                        <NumberInput
                                            value={form.simulation_start_time}
                                            min={0} max={SIMULATION_START_TIME_MAX_S} step={300}
                                            onChange={v => set('simulation_start_time', v)}
                                        />
                                    </FieldRow>
                                    <FieldRow
                                        label="Pas de temps"
                                        hint="Entre 0 et 1 seconde"
                                    >
                                        <NumberInput
                                            value={form.time_step}
                                            min={TIME_STEP_MIN_S} max={TIME_STEP_MAX_S} step={TIME_STEP_MIN_S}
                                            onChange={v => set('time_step', v)}
                                        />
                                    </FieldRow>
                                </div>
                            )}

                            {section === 'budget' && (
                                <div className="space-y-5">
                                    <FieldRow label="Budget maximum" hint="En unités monétaires">
                                        <NumberInput
                                            value={form.max_budget}
                                            min={1_000_000} max={10_000_000_000} step={1}
                                            onChange={v => set('max_budget', v)}
                                        />
                                    </FieldRow>
                                    <FieldRow label="Coût par mètre de route" hint="Multiplié par le nombre de voies">
                                        <NumberInput
                                            value={form.base_cost_per_meter}
                                            min={0} max={10_000} step={50}
                                            onChange={v => set('base_cost_per_meter', v)}
                                        />
                                    </FieldRow>
                                    <hr className="border-gray-100" />
                                    <p className="text-xs font-medium text-gray-500 uppercase tracking-wide">Coût de base des nœuds</p>
                                    <FieldRow label="Intersection">
                                        <NumberInput
                                            value={form.intersection_cost}
                                            min={0} max={5_000_000} step={5_000}
                                            onChange={v => set('intersection_cost', v)}
                                        />
                                    </FieldRow>
                                    <FieldRow label="Zone habitation">
                                        <NumberInput
                                            value={form.habitation_cost}
                                            min={0} max={5_000_000} step={5_000}
                                            onChange={v => set('habitation_cost', v)}
                                        />
                                    </FieldRow>
                                    <FieldRow label="Zone travail">
                                        <NumberInput
                                            value={form.workplace_cost}
                                            min={0} max={5_000_000} step={5_000}
                                            onChange={v => set('workplace_cost', v)}
                                        />
                                    </FieldRow>
                                </div>
                            )}

                            {section === 'score' && (
                                <div className="space-y-5">
                                    <p className="text-sm text-gray-500">
                                        Les poids définissent l&apos;importance de chaque critère. Leur somme doit être égale à 1,0.
                                    </p>
                                    <FieldRow label="Temps de trajet">
                                        <WeightInput value={form.time_weight} onChange={v => set('time_weight', v)} />
                                    </FieldRow>
                                    <FieldRow label="Taux de réussite">
                                        <WeightInput value={form.success_weight} onChange={v => set('success_weight', v)} />
                                    </FieldRow>
                                    <FieldRow label="Pollution (CO₂)">
                                        <WeightInput value={form.pollution_weight} onChange={v => set('pollution_weight', v)} />
                                    </FieldRow>
                                    <FieldRow label="Infrastructure">
                                        <WeightInput value={form.infrastructure_weight} onChange={v => set('infrastructure_weight', v)} />
                                    </FieldRow>
                                    <div className={`flex items-center gap-2 pt-1 text-sm font-medium ${weightsValid ? 'text-green-600' : 'text-red-500'}`}>
                                        <span>Somme :</span>
                                        <span>{weightSum.toFixed(2)}</span>
                                        {!weightsValid && <span className="text-xs font-normal">(doit être égale à 1,0)</span>}
                                    </div>
                                </div>
                            )}
                        </>
                    )}
                </div>

                {/* Footer */}
                <div className="px-8 pb-8 pt-4 flex items-center justify-between border-t border-gray-100">
                    {error && <p className="text-red-500 text-sm">{error}</p>}
                    {!error && <span />}
                    <div className="flex gap-3">
                        <button
                            onClick={onClose}
                            className="px-5 py-2 rounded-xl text-sm text-gray-600 hover:bg-gray-100 transition-colors"
                        >
                            Annuler
                        </button>
                        <button
                            onClick={handleSave}
                            disabled={!form || !weightsValid || saving}
                            className="px-5 py-2 rounded-xl text-sm bg-black text-white hover:bg-neutral-800 transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
                        >
                            {saving ? 'Sauvegarde…' : 'Sauvegarder'}
                        </button>
                    </div>
                </div>
            </div>
        </div>
    );
}

function FieldRow({ label, hint, children }: { label: string; hint?: string; children: React.ReactNode }) {
    return (
        <div className="flex items-center justify-between gap-4">
            <div>
                <p className="text-sm font-medium text-gray-800">{label}</p>
                {hint && <p className="text-xs text-gray-400">{hint}</p>}
            </div>
            {children}
        </div>
    );
}

function NumberInput({
    value, min, max, step, onChange, suffix,
}: {
    value: number; min: number; max: number; step: number;
    onChange: (v: number) => void; suffix?: string;
}) {
    return (
        <div className="flex items-center gap-1">
            <input
                type="number"
                value={value}
                min={min}
                max={max}
                step={step}
                onChange={e => onChange(Number(e.target.value))}
                className="w-36 text-right border border-gray-200 rounded-lg px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-black/10 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
            />
            {suffix && <span className="text-xs text-gray-400">{suffix}</span>}
        </div>
    );
}

function WeightInput({ value, onChange }: { value: number; onChange: (v: number) => void }) {
    return (
        <input
            type="number"
            value={value}
            min={0}
            max={1}
            step={0.05}
            onChange={e => onChange(Number(e.target.value))}
            className="w-24 text-right border border-gray-200 rounded-lg px-3 py-1.5 text-sm focus:outline-none focus:ring-2 focus:ring-black/10 [appearance:textfield] [&::-webkit-outer-spin-button]:appearance-none [&::-webkit-inner-spin-button]:appearance-none"
        />
    );
}
