import { Viewport, IViewportOptions, IWheelOptions } from 'pixi-viewport';

export class CustomViewport extends Viewport {
	constructor(
		options: IViewportOptions & {
			decelerate?: boolean;
			drag?: boolean;
			pinch?: boolean;
			wheel?: boolean | IWheelOptions;
		}
	) {
		const { decelerate, drag, pinch, wheel, ...rest } = options;
		super(rest);
		if (decelerate) this.decelerate();
		if (drag) this.drag();
		if (pinch) this.pinch();
		if (wheel) {
			if (typeof wheel === 'boolean') {
				this.wheel();
			} else {
				this.wheel(wheel);
			}
		}
	}
}
