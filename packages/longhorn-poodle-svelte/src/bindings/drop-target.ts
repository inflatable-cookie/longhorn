import type { TransferCommitSelector } from "@inflatable-cookie/longhorn/transfer";

export type DropSelector =
  | { readonly kind: "explicit_zone"; readonly dropZoneId: string }
  | { readonly kind: "screen_point" };

export function resolveDropSelector(
  selector: DropSelector,
  screenPoint: (() => { readonly x: number; readonly y: number }) | undefined,
): TransferCommitSelector {
  if (selector.kind === "explicit_zone") {
    return { kind: "explicit_zone", drop_zone_id: selector.dropZoneId };
  }
  const point = screenPoint?.();
  if (point === undefined || !Number.isSafeInteger(point.x) || !Number.isSafeInteger(point.y)) {
    throw new RangeError("drop screen point must use integer screen DIPs");
  }
  return { kind: "screen_point", point: { x: point.x, y: point.y } };
}

export function reportError(reporter: (error: unknown) => void, error: unknown): void {
  try {
    reporter(error);
  } catch {
    // Reporting failure must not escape an event callback.
  }
}

export function notify<Value>(
  listener: ((value: Value) => void) | undefined,
  value: Value,
  reporter: (error: unknown) => void,
): void {
  try {
    listener?.(value);
  } catch (error) {
    reportError(reporter, error);
  }
}
