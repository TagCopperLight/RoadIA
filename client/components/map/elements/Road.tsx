import { FederatedPointerEvent } from 'pixi.js';
import { MapNode, EditTool } from '../types';

interface RoadProps {
	start: MapNode;
	end: MapNode;
	selected?: boolean;
	activeTool?: EditTool;
	onSelect?: (e: FederatedPointerEvent) => void;
}

export function Road({ start, end, selected = false, activeTool, onSelect }: RoadProps) {
	const width = 15;
	const isInteractive = activeTool === 'select';

	return (
		<pixiGraphics
			eventMode={isInteractive ? 'static' : 'none'}
			cursor={isInteractive ? 'pointer' : 'default'}
			onClick={onSelect}
			draw={(graphics) => {
				graphics.clear();

				const dx = end.x - start.x;
				const dy = end.y - start.y;
				const length = Math.sqrt(dx * dx + dy * dy);
				const angle = Math.atan2(dy, dx);

				graphics.position.set(start.x, start.y);
				graphics.rotation = angle;

				const fillColor = selected ? 0xddcc00 : 0x888888;
				graphics.setFillStyle({ color: fillColor });
				graphics.rect(0, -width / 2, length, width);
				graphics.fill();

				graphics.setStrokeStyle({ color: 'white' });
				graphics.moveTo(0, 0);
				graphics.lineTo(length, 0);
				graphics.stroke();
			}}
		/>
	);
}
