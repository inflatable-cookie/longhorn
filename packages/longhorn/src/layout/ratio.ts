import { LAYOUT_RATIO_ONE_MILLIONTHS } from "./generated/protocol.ts";
import type { LayoutRatio } from "./generated/protocol.ts";

export function isLayoutRatio(value: unknown): value is LayoutRatio {
  return typeof value === "number" && ratioInRange(value);
}

export function layoutRatioFromMillionths(value: number): LayoutRatio {
  if (!ratioInRange(value)) {
    throw new RangeError(
      `layout ratio must be an integer from 0 through ${LAYOUT_RATIO_ONE_MILLIONTHS}; received ${String(value)}`,
    );
  }
  return value;
}

export function layoutRatioToUnitInterval(value: LayoutRatio): number {
  return layoutRatioFromMillionths(value) / LAYOUT_RATIO_ONE_MILLIONTHS;
}

function ratioInRange(value: number): boolean {
  return Number.isSafeInteger(value) && value >= 0 && value <= LAYOUT_RATIO_ONE_MILLIONTHS;
}
