import {
  BRIDGE_RETRY_CLASSES,
  type BridgeRetryClass,
} from "../generated/protocol.ts";

export const MAXIMUM_BRIDGE_AUTOMATIC_RETRIES = 64;

export interface BridgeRuntimeClock {
  now(): number;
}

export interface BridgeRuntimeBackoff {
  delay(retryClass: BridgeRetryClass, attempt: number): number;
}

export class BridgeRuntimeError extends Error {
  readonly code:
    | "invalid_transition"
    | "required_authority_unavailable"
    | "invalid_retry_limit"
    | "invalid_retry_class"
    | "invalid_time"
    | "retry_not_due";

  constructor(
    code: BridgeRuntimeError["code"],
    detail: string,
  ) {
    super(`bridge runtime: ${detail}`);
    this.name = "BridgeRuntimeError";
    this.code = code;
  }
}

export function checkedRetryLimit(value: number): number {
  if (
    !Number.isSafeInteger(value) ||
    value < 0 ||
    value > MAXIMUM_BRIDGE_AUTOMATIC_RETRIES
  ) {
    throw new BridgeRuntimeError(
      "invalid_retry_limit",
      `retry limit must be 0..${MAXIMUM_BRIDGE_AUTOMATIC_RETRIES}`,
    );
  }
  return value;
}

export function checkedRetryClass(
  value: BridgeRetryClass,
): BridgeRetryClass {
  if (!BRIDGE_RETRY_CLASSES.includes(value)) {
    throw new BridgeRuntimeError(
      "invalid_retry_class",
      "unknown retry class",
    );
  }
  return value;
}

export function checkedMonotonic(value: number): number {
  if (!Number.isSafeInteger(value) || value < 0) {
    throw new BridgeRuntimeError(
      "invalid_time",
      "clock must return a nonnegative safe integer",
    );
  }
  return value;
}

export function checkedDeadline(at: number, delay: number): number {
  const checkedDelay = checkedMonotonic(delay);
  const value = at + checkedDelay;
  if (!Number.isSafeInteger(value)) {
    throw new BridgeRuntimeError("invalid_time", "retry deadline overflow");
  }
  return value;
}

export function checkedIncrement(value: number): number {
  const next = value + 1;
  if (!Number.isSafeInteger(next)) {
    throw new BridgeRuntimeError(
      "invalid_time",
      "transition sequence exhausted",
    );
  }
  return next;
}
