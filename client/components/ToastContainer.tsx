'use client';

import { Toast, ToastType } from '@/hooks/useToast';

interface ToastContainerProps {
	toasts: Toast[];
	onRemove: (id: string) => void;
}

const bgColor: Record<ToastType, string> = {
	success: 'bg-green-600',
	error: 'bg-red-600',
	info: 'bg-blue-600',
	warning: 'bg-yellow-600',
};

const icon: Record<ToastType, string> = {
	success: '✓',
	error: '✕',
	info: 'ℹ',
	warning: '⚠',
};

export default function ToastContainer({ toasts, onRemove }: ToastContainerProps) {
	return (
		<div className="fixed bottom-4 right-4 space-y-2 max-w-sm z-50">
			{toasts.map((toast) => (
				<div
					key={toast.id}
					className={`${bgColor[toast.type]} text-white px-4 py-3 rounded-lg shadow-lg flex items-center justify-between gap-3 animate-slide-in`}
				>
					<div className="flex items-center gap-2">
						<span className="text-lg font-bold">{icon[toast.type]}</span>
						<span>{toast.message}</span>
					</div>
					<button
						onClick={() => onRemove(toast.id)}
						className="text-white/70 hover:text-white text-lg leading-none"
					>
						×
					</button>
				</div>
			))}
		</div>
	);
}
