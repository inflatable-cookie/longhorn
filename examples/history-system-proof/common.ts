import {
  createDirectHistoryPort,
  HistoryController,
  type HistoryChangedEvent,
  type HistoryNavigationCommand,
  type HistoryNavigationResult,
  type HistoryPageCommand,
  type HistoryPageSnapshot,
  type HistoryPort,
  type HistorySnapshot,
} from "@inflatable-cookie/longhorn-history";

export interface RendererFixture {
  readonly initialSnapshot: HistorySnapshot;
  readonly initialPage: HistoryPageSnapshot;
  readonly navigationResult: HistoryNavigationResult;
  readonly committedPage: HistoryPageSnapshot;
  readonly changedEvent: HistoryChangedEvent;
  readonly expectedPublicTrace: PublicTrace;
}

export interface PublicTrace {
  readonly revision: number;
  readonly undoDepth: number;
  readonly redoDepth: number;
  readonly currentEntryId: string | null;
  readonly entries: readonly {
    readonly entryId: string;
    readonly position: "past" | "current" | "future";
  }[];
}

export interface ControllerHarness {
  readonly controller: HistoryController;
  readonly port: HistoryPort;
  readonly emit: (event: HistoryChangedEvent) => void;
  readonly counters: {
    snapshot: number;
    page: number;
    navigate: number;
    unlisten: number;
  };
}

export function createControllerHarness(
  fixture: RendererFixture,
): ControllerHarness {
  let snapshot = fixture.initialSnapshot;
  let page = fixture.initialPage;
  const listeners = new Set<(event: unknown) => void>();
  let plan = 0;
  const counters = {
    snapshot: 0,
    page: 0,
    navigate: 0,
    unlisten: 0,
  };
  const port = createDirectHistoryPort({
    async snapshot() {
      counters.snapshot += 1;
      return structuredClone(snapshot);
    },
    async page(command: HistoryPageCommand) {
      counters.page += 1;
      assertPageCommand(command, snapshot);
      return structuredClone(page);
    },
    async navigate(command: HistoryNavigationCommand) {
      counters.navigate += 1;
      if (command.expectedRevision !== snapshot.summary.revision) {
        throw new Error("controller emitted a stale navigation command");
      }
      const result = structuredClone(fixture.navigationResult);
      snapshot = result.snapshot;
      if (result.status === "committed") page = fixture.committedPage;
      return result;
    },
    listen(listener) {
      listeners.add(listener);
      return () => {
        listeners.delete(listener);
        counters.unlisten += 1;
      };
    },
    nextPlanId() {
      plan += 1;
      return `plan:renderer-${plan}`;
    },
  });
  return {
    controller: new HistoryController({ port, pageSize: 50 }),
    port,
    emit(event) {
      for (const listener of listeners) listener(structuredClone(event));
    },
    counters,
  };
}

export function publicTrace(controller: HistoryController): PublicTrace {
  const snapshot = controller.snapshot;
  if (snapshot === undefined) throw new Error("missing controller snapshot");
  return {
    revision: snapshot.summary.revision,
    undoDepth: snapshot.summary.undoDepth,
    redoDepth: snapshot.summary.redoDepth,
    currentEntryId: snapshot.summary.currentEntryId,
    entries: controller.entries.map(({ entryId, position }) => ({
      entryId,
      position,
    })),
  };
}

export function rendererFixture(value: unknown): RendererFixture {
  return value as RendererFixture;
}

export function equalJson(left: unknown, right: unknown): boolean {
  return JSON.stringify(canonical(left)) === JSON.stringify(canonical(right));
}

export async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}

function canonical(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(canonical);
  if (value !== null && typeof value === "object") {
    return Object.fromEntries(
      Object.entries(value)
        .sort(([left], [right]) => left.localeCompare(right))
        .map(([key, entry]) => [key, canonical(entry)]),
    );
  }
  return value;
}

function assertPageCommand(
  command: HistoryPageCommand,
  snapshot: HistorySnapshot,
): void {
  if (
    command.authorityEpoch !== snapshot.authorityEpoch ||
    command.historyId !== snapshot.summary.historyId ||
    command.expectedRevision !== snapshot.summary.revision
  ) {
    throw new Error("controller page command lost authoritative identity");
  }
}
