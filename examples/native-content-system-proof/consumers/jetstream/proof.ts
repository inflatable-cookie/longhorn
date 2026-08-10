import {
  assertValidNativeContentSnapshot,
  type NativeContentSnapshot,
} from "@inflatable-cookie/longhorn/native-content";

import fixture from "./fixture.json";
import {
  nativeContentTrace,
  type NativeContentTraceFixture,
} from "../../common.ts";

const typed = fixture as NativeContentTraceFixture;
for (const snapshot of typed.snapshots) {
  assertValidNativeContentSnapshot(snapshot);
  if (snapshot.desired.capabilities.mechanism !== "backing_surface") {
    throw new Error("Jetstream fixture is not backing-surface coordination");
  }
}

console.log(JSON.stringify({
  publicTrace: nativeContentTrace(
    typed.snapshots as readonly NativeContentSnapshot[],
  ),
}));
