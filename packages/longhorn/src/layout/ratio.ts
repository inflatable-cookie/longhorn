import type { LayoutRatio } from "./generated/protocol.ts";

export const LAYOUT_RATIO_ONE_MILLIONTHS = 1_000_000;

export function layoutRatioFromMillionths(value: number): LayoutRatio {
  if (
    !Number.isSafeInteger(value) ||
    value < 0 ||
    value > LAYOUT_RATIO_ONE_MILLIONTHS
  ) {
    throw new RangeError(
      `layout ratio must be an integer from 0 through ${LAYOUT_RATIO_ONE_MILLIONTHS}; received ${String(value)}`,
    );
  }
  return value;
}

export function layoutRatioToUnitInterval(value: LayoutRatio): number {
  return (
    layoutRatioFromMillionths(value) / LAYOUT_RATIO_ONE_MILLIONTHS
  );
}
