import type { NativeContentSnapshot } from "@longhorn/native-content";

export interface NativeContentTraceFixture {
  readonly snapshots: readonly NativeContentSnapshot[];
}

export function nativeContentTrace(
  snapshots: readonly NativeContentSnapshot[],
) {
  return snapshots.map((snapshot) => ({
    cursor: {
      generation: snapshot.cursor.attach_generation,
      desiredRevision: snapshot.cursor.desired_revision,
      observedRevision: snapshot.cursor.observed_revision,
    },
    mechanism: snapshot.desired.capabilities.mechanism,
    desired: {
      viewport: snapshot.desired.viewport,
      scale: snapshot.desired.scale,
      visibility: snapshot.desired.visibility,
      focus: snapshot.desired.focus,
      inputRouting: snapshot.desired.input_routing,
    },
    observed: {
      lifecycle: snapshot.observed.lifecycle,
      readiness: snapshot.observed.readiness,
      visibility: snapshot.observed.visibility,
      focus: snapshot.observed.focus,
      geometry: snapshot.observed.geometry.kind,
      inputRouting: snapshot.observed.input_routing,
    },
  }));
}
