import {
  assertCompatibleNativeContentSnapshot,
  type NativeContentSnapshot,
} from "@inflatable-cookie/longhorn-native-content";

import fixture from "./fixture.json";
import {
  nativeContentTrace,
  type NativeContentTraceFixture,
} from "../../common.ts";

const typed = fixture as NativeContentTraceFixture;
for (const snapshot of typed.snapshots) {
  assertCompatibleNativeContentSnapshot(snapshot);
  if (snapshot.desired.capabilities.mechanism !== "child_view") {
    throw new Error("Nucleus fixture is not child-view coordination");
  }
}

console.log(JSON.stringify({
  publicTrace: nativeContentTrace(
    typed.snapshots as readonly NativeContentSnapshot[],
  ),
}));
