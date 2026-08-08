import { OperationSession } from "../../src/operation/svelte.ts";
import type {
  OperationCancellationCommand,
  OperationCancellationResult,
  OperationChangedEvent,
  OperationEntryProjection,
  OperationMutationCommand,
  OperationMutationResult,
  OperationSnapshot,
  OperationSnapshotQuery,
} from "../../../longhorn/src/operation/generated/protocol.ts";
import type { OperationPort, OperationUnlisten } from "../../../longhorn/src/operation/ports.ts";
import { cloned, fixture } from "../../../longhorn/tests/operation/support.ts";

export class MountedOperationPort implements OperationPort {
  readonly listeners = new Set<(event: unknown) => void>();
  readonly cancellations: OperationCancellationCommand[] = [];
  readonly mutations: OperationMutationCommand[] = [];
  snapshotValue: OperationSnapshot;
  unlistenCount = 0;
  #request = 0;

  constructor(snapshot: OperationSnapshot) {
    this.snapshotValue = cloned(snapshot);
  }

  async snapshot(query: OperationSnapshotQuery): Promise<unknown> {
    return { requestId: query.requestId, snapshot: cloned(this.snapshotValue) };
  }

  async cancel(command: OperationCancellationCommand): Promise<unknown> {
    this.cancellations.push(command);
    const result = cancelResult(this.snapshotValue, command);
    this.snapshotValue = cloned(result.snapshot);
    return result;
  }

  async mutate(command: OperationMutationCommand): Promise<unknown> {
    this.mutations.push(command);
    const result = dismissResult(this.snapshotValue, command);
    this.snapshotValue = cloned(result.snapshot);
    return result;
  }

  listen(listener: (event: unknown) => void): OperationUnlisten {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
      this.unlistenCount += 1;
    };
  }

  nextRequestId(): string {
    this.#request += 1;
    return `request:mounted-${this.#request}`;
  }

  publish(snapshot: OperationSnapshot): void {
    const previousCatalogueRevision = this.snapshotValue.catalogueRevision;
    this.snapshotValue = cloned(snapshot);
    const event: OperationChangedEvent = {
      protocolVersion: 1,
      requestId: `request:mounted-event-${snapshot.catalogueRevision}`,
      authority: snapshot.authority,
      previousCatalogueRevision,
      committedCatalogueRevision: snapshot.catalogueRevision,
      operationId: null,
      kind: "mutation",
    };
    for (const listener of this.listeners) listener(event);
  }
}

export function soundcheckSnapshot(): OperationSnapshot {
  return cloned(fixture.mutationResults[1]!.snapshot);
}

export function loopholeSnapshot(): OperationSnapshot {
  const snapshot = cloned(fixture.mutationResults[2]!.snapshot);
  snapshot.catalogueRevision = 8;
  snapshot.recent[0] = operation({
    ...snapshot.recent[0]!,
    operationId: "operation:render-complete",
    kindId: "loophole.render",
    label: "Render opening titles",
    state: "succeeded",
    lastChangedCatalogueRevision: 7,
  });
  snapshot.active = [
    operation({
      ...snapshot.recent[0]!,
      operationId: "operation:render-running",
      label: "Render final sequence",
      state: "running",
      revision: 2,
      lastChangedCatalogueRevision: 8,
      progress: {
        sequence: 2,
        overall: { kind: "normalized", value: 0.65 },
        phase: null,
      },
    }),
    operation({
      ...snapshot.recent[0]!,
      operationId: "operation:render-queued",
      label: "Render trailer",
      state: "queued",
      revision: 0,
      lastChangedCatalogueRevision: 8,
      progress: {
        sequence: 0,
        overall: { kind: "indeterminate" },
        phase: null,
      },
    }),
  ];
  return snapshot;
}

export function createMountedSession(snapshot = soundcheckSnapshot()) {
  const port = new MountedOperationPort(snapshot);
  const session = new OperationSession({ port });
  return { port, session };
}

function operation(value: OperationEntryProjection): OperationEntryProjection {
  return value;
}

function dismissResult(
  snapshot: OperationSnapshot,
  command: OperationMutationCommand,
): OperationMutationResult {
  if (command.kind !== "dismiss") throw new Error("fixture only supports dismissal");
  const next = cloned(snapshot);
  const index = next.recent.findIndex(({ operationId }) => operationId === command.operationId);
  const [removed] = next.recent.splice(index, 1);
  if (removed === undefined) throw new Error("missing retained operation");
  next.catalogueRevision += 1;
  next.retainedTerminalEncodedWeight = next.recent.reduce(
    (total, entry) => total + entry.encodedMetadataWeight,
    0,
  );
  return {
    status: "committed",
    requestId: command.requestId,
    snapshot: next,
    receipt: {
      kind: "dismissed",
      removed: {
        operationId: removed.operationId,
        sequence: removed.sequence,
        encodedWeight: removed.encodedMetadataWeight,
        reason: "dismissed",
      },
      previousCatalogueRevision: snapshot.catalogueRevision,
      committedCatalogueRevision: next.catalogueRevision,
    },
  };
}

function cancelResult(
  snapshot: OperationSnapshot,
  command: OperationCancellationCommand,
): OperationCancellationResult {
  const next = cloned(snapshot);
  const operation = next.active.find(
    ({ operationId }) => operationId === command.operationId,
  );
  if (operation === undefined) throw new Error("missing active operation");
  const previousState = operation.state;
  const previousOperationRevision = operation.revision;
  const previousCatalogueRevision = next.catalogueRevision;
  operation.state = "cancelling";
  operation.revision += 1;
  next.catalogueRevision += 1;
  operation.lastChangedCatalogueRevision = next.catalogueRevision;
  return {
    status: "committed",
    requestId: command.requestId,
    snapshot: next,
    receipt: {
      operationId: operation.operationId,
      outcome: "accepted",
      previousState,
      committedState: operation.state,
      previousOperationRevision,
      committedOperationRevision: operation.revision,
      previousCatalogueRevision,
      committedCatalogueRevision: next.catalogueRevision,
      evicted: [],
    },
    executorDispatch: { kind: "requested" },
  };
}
