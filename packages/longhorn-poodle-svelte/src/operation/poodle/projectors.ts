import {
  OPERATION_STATE_LABELS,
  type OperationEntryProjection,
  type OperationOverallProgressProjection,
  type OperationStateProjection,
} from "@inflatable-cookie/longhorn/operation";

export type OperationStatusTone =
  | "neutral"
  | "info"
  | "success"
  | "warning"
  | "danger"
  | "pending";

export interface OperationProgressView {
  readonly indeterminate: boolean;
  readonly value: number | null;
  readonly max: number;
  readonly valueText: string | null;
}

const stateTones: Record<OperationStateProjection, OperationStatusTone> = {
  queued: "pending",
  running: "info",
  cancelling: "warning",
  succeeded: "success",
  failed: "danger",
  cancelled: "neutral",
  interrupted: "warning",
};

/**
 * Generated from Rust, where the wording lives on the enum that owns it.
 * A label added to the union and missing from the map is a type error here
 * rather than a blank on screen. See memo 022, D2.
 */
export function operationStateLabel(state: OperationStateProjection): string {
  return OPERATION_STATE_LABELS[state];
}

export function operationStatusTone(
  state: OperationStateProjection,
): OperationStatusTone {
  return stateTones[state];
}

export function operationProgressView(
  progress: OperationOverallProgressProjection,
): OperationProgressView {
  switch (progress.kind) {
    case "indeterminate":
      return { indeterminate: true, value: null, max: 100, valueText: null };
    case "units":
      return {
        indeterminate: false,
        value: progress.completed,
        max: progress.total,
        valueText: `${progress.completed} of ${progress.total}`,
      };
    case "normalized":
      return {
        indeterminate: false,
        value: progress.value,
        max: 1,
        valueText: `${Math.round(progress.value * 100)}%`,
      };
  }
}

export function canCancelOperation(
  operation: OperationEntryProjection,
): boolean {
  return (
    operation.cancellationSupport === "supported" &&
    (operation.state === "queued" || operation.state === "running")
  );
}
