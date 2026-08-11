// Where macOS puts the traffic lights under `titleBarStyle: "Overlay"`.
//
// With the overlay style the window keeps its native frame -- rounded corners,
// shadow, real traffic lights -- and the application draws the bar behind them.
// The lights then have to be placed against a header only the application knows
// the height of, and every consumer arrived at the same convention by hand:
// hard left at x 18, vertically centred on the header.
//
// Observed before this was written down, across five applications:
//
//     header 35.2px -> y 18      header 48px -> y 25
//     header 48px   -> y 25      header 62px -> y 31
//     header 72px   -> y 37
//
// Tauri's `trafficLightPosition.y` is the centre of the button group rather
// than its top edge, which is why these are half the header rather than
// half-minus-the-button. The one comment that recorded any of this lives in
// figmatic's TitleBar.svelte: "keep it at half this height (62px -> y: 31)".
//
// This is set when the window is created -- `tauri.conf.json`, or
// `WebviewWindowBuilder` -- and cannot be changed afterwards. Tauri 2.10.3 has
// `set_traffic_light_position` on the `tauri-runtime` dispatch trait but does
// not surface it on `Window` or `WebviewWindow`, and the JavaScript API only
// accepts `trafficLightPosition` in `WindowOptions`. So nothing can move these
// in response to a header that resizes at runtime.

/** Hard left, the same in every consumer. */
export const TRAFFIC_LIGHT_INSET_X = 18;

/**
 * The optical correction. Geometric centring sits a pixel high against the
 * window's top border, so the group is nudged down by one. This is a judgement
 * about how it looks, not a measurement, which is why it is named rather than
 * folded into the arithmetic.
 */
export const TRAFFIC_LIGHT_OPTICAL_OFFSET = 1;

export interface TrafficLightPosition {
  readonly x: number;
  readonly y: number;
}

/**
 * Returns the `trafficLightPosition` for a titlebar of the given height.
 *
 * `headerHeight` is in logical pixels — the CSS height of the bar the lights
 * sit in, so a `2.2rem` header is `35.2`, not `2.2`.
 *
 * Applies only where the titlebar spans the window's left edge. A layout whose
 * bar starts after a rail leaves the lights over the rail instead, and should
 * pass the height of whatever they actually sit against.
 */
export function trafficLightPosition(headerHeight: number): TrafficLightPosition {
  if (!Number.isFinite(headerHeight) || headerHeight <= 0) {
    throw new RangeError(
      `titlebar height must be a positive number of logical pixels, got ${headerHeight}`,
    );
  }
  return {
    x: TRAFFIC_LIGHT_INSET_X,
    y: Math.round(headerHeight / 2) + TRAFFIC_LIGHT_OPTICAL_OFFSET,
  };
}
