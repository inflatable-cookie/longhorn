import {
  HISTORY_CHANGED_EVENT,
  createTauriHistoryPort,
} from "@inflatable-cookie/longhorn-tauri/history";
import {
  HistoryController,
  type HistoryChangedEvent,
  type HistoryNavigationCommand,
  type HistoryPageCommand,
} from "@inflatable-cookie/longhorn/history";

import {
  equalJson,
  publicTrace,
  rendererFixture,
  settle,
} from "../../common.ts";
import fixtureJson from "./fixture.json";

const fixture = rendererFixture(fixtureJson);
let snapshot = fixture.initialSnapshot;
let page = fixture.initialPage;
let listener: ((event: unknown) => void) | undefined;
let unlisten = 0;
let snapshots = 0;
let plan = 0;

const transport = {
  async invoke(
    command: string,
    args: Record<string, unknown>,
  ): Promise<unknown> {
    if (command === "longhorn_history_snapshot") {
      snapshots += 1;
      return structuredClone(snapshot);
    }
    if (command === "longhorn_history_page") {
      const request = args?.command as HistoryPageCommand;
      if (request.expectedRevision !== snapshot.summary.revision) {
        throw new Error("Tauri page request was stale");
      }
      return structuredClone(page);
    }
    if (command === "longhorn_history_navigate") {
      const request = args?.command as HistoryNavigationCommand;
      if (request.expectedRevision !== snapshot.summary.revision) {
        throw new Error("Tauri navigation request was stale");
      }
      snapshot = fixture.navigationResult.snapshot;
      page = fixture.committedPage;
      return structuredClone(fixture.navigationResult);
    }
    throw new Error(`unexpected invoke command ${command}`);
  },
  async listen(
    event: string,
    next: (event: unknown) => void,
  ): Promise<() => void> {
    if (event !== HISTORY_CHANGED_EVENT) {
      throw new Error(`unexpected event ${event}`);
    }
    listener = next;
    return () => {
      listener = undefined;
      unlisten += 1;
    };
  },
};

const port = createTauriHistoryPort({
  transport,
  nextPlanId() {
    plan += 1;
    return `plan:tauri-${plan}`;
  },
});

const controller = new HistoryController({ port, pageSize: 50 });
await controller.start();
if (
  controller.status.kind !== "ready" ||
  !controller.entries.some(({ position }) => position === "future")
) {
  throw new Error("rich controller lost authoritative future entries");
}
await controller.undo();
const trace = publicTrace(controller);
if (!equalJson(trace, fixture.expectedPublicTrace)) {
  throw new Error(
    `rich Rust and renderer traces diverged: ${JSON.stringify(trace)} != ${JSON.stringify(fixture.expectedPublicTrace)}`,
  );
}

const snapshotCallsBeforeGap = snapshots;
listener?.({
  ...fixture.changedEvent,
  authorityEpoch: fixture.changedEvent.authorityEpoch + 1,
} satisfies HistoryChangedEvent);
await settle();
if (snapshots <= snapshotCallsBeforeGap) {
  throw new Error("authority epoch gap did not trigger refresh");
}

await controller.stop();
if (unlisten !== 1 || listener !== undefined) {
  throw new Error("rich controller leaked its Tauri listener");
}

console.log(
  JSON.stringify({
    shape: "loophole",
    publicTrace: trace,
    transport: "tauri",
    futureAuthoritative: true,
    eventGapRefresh: true,
    teardown: { unlisten },
  }),
);
