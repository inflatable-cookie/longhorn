import type {
  DesiredVisibility,
  VisibilityReasonId,
} from "@longhorn/native-content";

export interface NativeContentVisibilityInhibitor {
  readonly reason: VisibilityReasonId;
  readonly active: boolean;
}

/** Resolves explicit consumer policy. Earlier inhibitors have precedence. */
export function resolveNativeContentVisibility(
  inhibitors: readonly NativeContentVisibilityInhibitor[],
): DesiredVisibility {
  const inhibitor = inhibitors.find(({ active }) => active);
  return inhibitor === undefined
    ? { state: "visible" }
    : { state: "hidden", reason: inhibitor.reason };
}
