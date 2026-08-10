import { BRIDGE_FIELDS } from "../generated/fields.ts";
import type {
  BridgeCancellationReceipt,
  BridgeJobTerminalEvent,
  BridgeProgressEvent,
} from "../generated/protocol.ts";
import {
  type BridgeCodec,
  incompatible,
  opaqueId,
  record,
} from "./base.ts";
import { parseBridgeFailure } from "./operations.ts";

export function parseBridgeProgressEvent<P>(
  value: unknown,
  progress: BridgeCodec<P>,
): BridgeProgressEvent<P> {
  const source = record(value, BRIDGE_FIELDS.BridgeProgressEvent);
  return {
    requestId: opaqueId(source.requestId),
    jobId: opaqueId(source.jobId),
    progress: progress.parse(source.progress),
  };
}

export function parseBridgeJobTerminalEvent<S, D>(
  value: unknown,
  success: BridgeCodec<S>,
  details: BridgeCodec<D>,
): BridgeJobTerminalEvent<S, D> {
  const source = record(value, BRIDGE_FIELDS.BridgeJobTerminalEvent);
  return {
    requestId: opaqueId(source.requestId),
    jobId: opaqueId(source.jobId),
    outcome: parseTerminalOutcome(source.outcome, success, details),
  };
}

export function parseBridgeCancellationReceipt<D>(
  value: unknown,
  details: BridgeCodec<D>,
): BridgeCancellationReceipt<D> {
  const source = record(value, BRIDGE_FIELDS.BridgeCancellationReceipt);
  return {
    requestId: opaqueId(source.requestId),
    targetRequestId: opaqueId(source.targetRequestId),
    jobId: opaqueId(source.jobId),
    outcome: parseCancellationOutcome(source.outcome, details),
  };
}

function parseTerminalOutcome<S, D>(
  value: unknown,
  success: BridgeCodec<S>,
  details: BridgeCodec<D>,
): BridgeJobTerminalEvent<S, D>["outcome"] {
  if (value === "cancelled") {
    return value;
  }
  const source = singleVariant(
    value,
    ["succeeded", "failed"],
    "unknown_job_outcome",
  );
  if ("succeeded" in source) {
    return { succeeded: success.parse(source.succeeded) };
  }
  return { failed: parseBridgeFailure(source.failed, details) };
}

function parseCancellationOutcome<D>(
  value: unknown,
  details: BridgeCodec<D>,
): BridgeCancellationReceipt<D>["outcome"] {
  if (
    value === "accepted" ||
    value === "alreadyTerminal" ||
    value === "unknown"
  ) {
    return value;
  }
  const source = singleVariant(
    value,
    ["rejected"],
    "unknown_cancellation_outcome",
  );
  return { rejected: parseBridgeFailure(source.rejected, details) };
}

function singleVariant(
  value: unknown,
  keys: readonly string[],
  code: "unknown_job_outcome" | "unknown_cancellation_outcome",
): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    incompatible(code, value);
  }
  const source = value as Record<string, unknown>;
  const present = Object.keys(source);
  if (present.length !== 1 || !keys.includes(present[0]!)) {
    incompatible(code, value);
  }
  return source;
}
