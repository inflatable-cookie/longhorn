import type {
  OperationEntryProjection,
  OperationOverallProgressProjection,
  OperationStateProjection,
} from "../generated/protocol.ts";

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

const stateLabels: Record<OperationStateProjection, string> = {
  queued: "Queued",
  running: "Running",
  cancelling: "Cancelling",
  succeeded: "Succeeded",
  failed: "Failed",
  cancelled: "Cancelled",
  interrupted: "Interrupted",
};

const stateTones: Record<OperationStateProjection, OperationStatusTone> = {
  queued: "pending",
  running: "info",
  cancelling: "warning",
  succeeded: "success",
  failed: "danger",
  cancelled: "neutral",
  interrupted: "warning",
};

export function operationStateLabel(state: OperationStateProjection): string {
  return stateLabels[state];
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
