import { describe, expect, test } from "bun:test";

import { OperationController } from "../../src/operation/controller.ts";
import type {
  OperationCancellationCommand,
  OperationCancellationResult,
  OperationChangedEvent,
  OperationMutationCommand,
  OperationMutationResult,
  OperationSnapshot,
  OperationSnapshotQuery,
} from "../../src/operation/generated/protocol.ts";
import type { OperationPort, OperationUnlisten } from "../../src/operation/ports.ts";
import { cloned, fixture } from "./support.ts";

class ControllerPort implements OperationPort {
  readonly listeners = new Set<(event: unknown) => void>();
  readonly cancellations: OperationCancellationCommand[] = [];
  readonly mutations: OperationMutationCommand[] = [];
  snapshotValue = cloned(fixture.mutationResults[1]!.snapshot);
  cancellation = deferred<OperationCancellationResult>();
  mutation = deferred<OperationMutationResult>();
  unlistenCount = 0;
  #request = 0;

  async snapshot(query: OperationSnapshotQuery): Promise<unknown> {
    return { requestId: query.requestId, snapshot: cloned(this.snapshotValue) };
  }

  async cancel(command: OperationCancellationCommand): Promise<unknown> {
    this.cancellations.push(command);
    const result = await this.cancellation.promise;
    return { ...cloned(result), requestId: command.requestId };
  }

  async mutate(command: OperationMutationCommand): Promise<unknown> {
    this.mutations.push(command);
    const result = await this.mutation.promise;
    return { ...cloned(result), requestId: command.requestId };
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
    return `request:controller-${this.#request}`;
  }

  publish(snapshot: OperationSnapshot): void {
    const previous = this.snapshotValue.catalogueRevision;
    this.snapshotValue = cloned(snapshot);
    const event: OperationChangedEvent = {
      protocolVersion: 1,
      requestId: `request:event-${snapshot.catalogueRevision}`,
      authority: snapshot.authority,
      previousCatalogueRevision: previous,
      committedCatalogueRevision: snapshot.catalogueRevision,
      operationId: null,
      kind: "mutation",
    };
    for (const listener of this.listeners) listener(event);
  }
}

describe("operation controller", () => {
  test("keeps cancellation pending by request and ignores a stale result", async () => {
    const port = new ControllerPort();
    const controller = new OperationController({ port });
    await controller.start();

    const cancellation = controller.cancel("operation:scan");
    expect(controller.pendingCommands).toEqual([
      {
        requestId: "request:controller-2",
        operationId: "operation:scan",
        kind: "cancellation",
      },
    ]);
    expect(port.cancellations[0]?.expectedOperationRevision).toBe(1);

    const terminal = cloned(fixture.mutationResults[2]!.snapshot);
    port.publish(terminal);
    await eventually(() => controller.snapshot?.catalogueRevision === 4);
    port.cancellation.resolve(cloned(fixture.cancellationResult));
    await cancellation;

    expect(controller.snapshot?.catalogueRevision).toBe(4);
    expect(controller.recent[0]?.state).toBe("succeeded");
    expect(controller.pendingCommands).toEqual([]);
    await controller.stop();
  });

  test("does not let an old-epoch dismissal overwrite a new authority epoch", async () => {
    const port = new ControllerPort();
    port.snapshotValue = cloned(fixture.mutationResults[2]!.snapshot);
    const controller = new OperationController({ port });
    await controller.start();

    const dismissal = controller.dismiss("operation:scan");
    expect(controller.pendingCommands).toEqual([
      {
        requestId: "request:controller-2",
        operationId: "operation:scan",
        kind: "dismissal",
      },
    ]);
    expect(port.mutations[0]?.kind).toBe("dismiss");
    const nextEpoch = cloned(port.snapshotValue);
    nextEpoch.authority.authorityEpoch += 1;
    nextEpoch.catalogueRevision = 1;
    nextEpoch.recent[0]!.authority.authorityEpoch += 1;
    nextEpoch.recent[0]!.label = "New authority result";
    nextEpoch.recent[0]!.lastChangedCatalogueRevision = 1;
    port.publish(nextEpoch);
    await eventually(() => controller.snapshot?.authority.authorityEpoch === 8);

    port.mutation.resolve(dismissalResult(fixture.mutationResults[2]!.snapshot));
    await dismissal;
    expect(controller.snapshot?.authority.authorityEpoch).toBe(8);
    expect(controller.recent[0]?.label).toBe("New authority result");
    expect(controller.pendingCommands).toEqual([]);
    await controller.stop();
  });

  test("teardown only releases observation and remount reloads host truth", async () => {
    const port = new ControllerPort();
    const controller = new OperationController({ port });
    await controller.start();
    expect(port.listeners.size).toBe(1);
    await controller.stop();
    expect(port.listeners.size).toBe(0);
    expect(port.unlistenCount).toBe(1);
    expect(port.cancellations).toEqual([]);

    port.snapshotValue = cloned(fixture.mutationResults[2]!.snapshot);
    await controller.start();
    expect(controller.recent[0]?.state).toBe("succeeded");
    expect(port.listeners.size).toBe(1);
    await controller.stop();
    expect(port.unlistenCount).toBe(2);
  });

  test("teardown closes a listener registration that resolves late exactly once", async () => {
    const registration = deferred<OperationUnlisten>();
    let unlistenCount = 0;
    let snapshotCount = 0;
    const port = new ControllerPort();
    const controller = new OperationController({
      port: {
        listen: () => registration.promise,
        snapshot: async (query) => {
          snapshotCount += 1;
          return { requestId: query.requestId, snapshot: port.snapshotValue };
        },
        mutate: (command) => port.mutate(command),
        cancel: (command) => port.cancel(command),
        nextRequestId: () => port.nextRequestId(),
      },
    });
    const start = controller.start();
    const stop = controller.stop();
    registration.resolve(() => {
      unlistenCount += 1;
    });
    await Promise.all([start, stop]);

    expect(unlistenCount).toBe(1);
    expect(snapshotCount).toBe(0);
    expect(controller.status.kind).toBe("idle");
  });
});

function dismissalResult(snapshot: OperationSnapshot): OperationMutationResult {
  const next = cloned(snapshot);
  const removed = next.recent.shift()!;
  next.catalogueRevision += 1;
  next.retainedTerminalEncodedWeight = 0;
  return {
    status: "committed",
    requestId: "request:dismiss-result",
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

function deferred<Value>(): {
  promise: Promise<Value>;
  resolve: (value: Value) => void;
} {
  let resolve!: (value: Value) => void;
  const promise = new Promise<Value>((next) => {
    resolve = next;
  });
  return { promise, resolve };
}

async function eventually(predicate: () => boolean): Promise<void> {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    if (predicate()) return;
    await Promise.resolve();
  }
  throw new Error("condition did not become true");
}
