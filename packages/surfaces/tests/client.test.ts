import { describe, expect, test } from "bun:test";
import type { EventTransport, Unlisten } from "@inflatable-cookie/longhorn-core";

import {
  SURFACE_CHANGED_EVENT,
  SURFACE_SNAPSHOT_COMMAND,
  SurfaceClient,
  type SurfaceChangedEvent,
  type SurfaceSnapshot,
} from "@inflatable-cookie/longhorn-surfaces";

describe("Surface snapshot subscription", () => {
  test("attaches before snapshot and closes a mutation race", async () => {
    const revision11 = snapshot(4, 11);
    const revision12 = snapshot(4, 12);
    const transport = new MockTransport(revision11);
    transport.onFirstSnapshot = () => {
      transport.current = revision12;
      transport.emit(changed(4, 12));
    };
    const observed: number[] = [];

    const subscription = new SurfaceClient(transport).subscribe((value) => {
      observed.push(value.revision);
    });
    await subscription.ready;

    expect(transport.operations.slice(0, 2)).toEqual([
      `listen:${SURFACE_CHANGED_EVENT}`,
      `invoke:${SURFACE_SNAPSHOT_COMMAND}`,
    ]);
    expect(observed).toEqual([11, 12]);
    expect(transport.snapshotInvocations).toBe(2);

    await subscription.dispose();
    await subscription.dispose();
    expect(transport.unlistenCalls).toBe(1);
  });

  test("ignores duplicate and stale hints and resyncs gaps and epochs", async () => {
    const transport = new MockTransport(snapshot(9, 20));
    const observed: Array<[number, number]> = [];
    const subscription = new SurfaceClient(transport).subscribe((value) => {
      observed.push([value.epoch, value.revision]);
    });
    await subscription.ready;

    transport.emit(changed(9, 20));
    transport.emit(changed(9, 19));
    await tick();
    expect(transport.snapshotInvocations).toBe(1);

    transport.current = snapshot(9, 24);
    transport.emit(changed(9, 24));
    await tick();
    transport.current = snapshot(8, 99);
    transport.emit(changed(8, 99));
    await tick();
    transport.current = snapshot(10, 1);
    transport.emit(changed(10, 1));
    await tick();

    expect(observed).toEqual([
      [9, 20],
      [9, 24],
      [10, 1],
    ]);
    expect(transport.snapshotInvocations).toBe(3);
    await subscription.dispose();
  });

  test("disposes a listener whose registration completes late", async () => {
    const transport = new MockTransport(snapshot(1, 0));
    transport.deferRegistration();
    const subscription = new SurfaceClient(transport).subscribe(() => {
      throw new Error("disposed subscription must not publish");
    });

    const disposal = subscription.dispose();
    transport.completeRegistration();
    await disposal;

    expect(transport.unlistenCalls).toBe(1);
    expect(transport.snapshotInvocations).toBe(0);
  });

  test("reports invalid event and load failures and tears down", async () => {
    const invalidEventTransport = new MockTransport(snapshot(1, 1));
    const failures: string[] = [];
    const subscription = new SurfaceClient(
      invalidEventTransport,
    ).subscribe(
      () => {},
      ({ phase }) => failures.push(phase),
    );
    await subscription.ready;
    invalidEventTransport.emitRaw({ protocol_version: 99 });
    await tick();

    expect(failures).toEqual(["event"]);
    expect(subscription.failures().map(({ phase }) => phase)).toEqual([
      "event",
    ]);
    expect(invalidEventTransport.unlistenCalls).toBe(1);

    const loadFailure = new Error("snapshot load failed");
    const failedTransport = new MockTransport(snapshot(1, 1));
    failedTransport.snapshotFailure = loadFailure;
    const failed = new SurfaceClient(failedTransport).subscribe(() => {});
    await expect(failed.ready).rejects.toBe(loadFailure);
    expect(failed.failures().map(({ phase }) => phase)).toEqual([
      "snapshot",
    ]);
    expect(failedTransport.unlistenCalls).toBe(1);
  });
});

class MockTransport implements EventTransport {
  current: SurfaceSnapshot;
  operations: string[] = [];
  snapshotInvocations = 0;
  unlistenCalls = 0;
  snapshotFailure: Error | undefined;
  onFirstSnapshot: (() => void) | undefined;
  #listener: ((payload: unknown) => void) | undefined;
  #registration:
    | {
        promise: Promise<Unlisten>;
        resolve: (unlisten: Unlisten) => void;
      }
    | undefined;

  constructor(current: SurfaceSnapshot) {
    this.current = current;
  }

  async invoke(command: string): Promise<unknown> {
    this.operations.push(`invoke:${command}`);
    if (command !== SURFACE_SNAPSHOT_COMMAND) {
      throw new Error(`unexpected command ${command}`);
    }
    this.snapshotInvocations += 1;
    if (this.snapshotFailure !== undefined) {
      throw this.snapshotFailure;
    }
    const result = this.current;
    if (this.snapshotInvocations === 1) {
      this.onFirstSnapshot?.();
    }
    return result;
  }

  listen(
    event: string,
    listener: (payload: unknown) => void,
  ): Promise<Unlisten> {
    this.operations.push(`listen:${event}`);
    this.#listener = listener;
    if (this.#registration !== undefined) {
      return this.#registration.promise;
    }
    return Promise.resolve(this.#unlisten);
  }

  emit(event: SurfaceChangedEvent): void {
    this.#listener?.(event);
  }

  emitRaw(event: unknown): void {
    this.#listener?.(event);
  }

  deferRegistration(): void {
    let resolve:
      | ((unlisten: Unlisten) => void)
      | undefined;
    const promise = new Promise<Unlisten>((accept) => {
      resolve = accept;
    });
    this.#registration = {
      promise,
      resolve: (unlisten) => resolve?.(unlisten),
    };
  }

  completeRegistration(): void {
    this.#registration?.resolve(this.#unlisten);
  }

  #unlisten = (): void => {
    this.unlistenCalls += 1;
    this.#listener = undefined;
  };
}

function snapshot(epoch: number, revision: number): SurfaceSnapshot {
  return {
    protocol_version: 1,
    epoch,
    revision,
    document: {
      revision,
      surfaces: [],
      windows: [],
    },
  };
}

function changed(epoch: number, revision: number): SurfaceChangedEvent {
  return {
    protocol_version: 1,
    epoch,
    revision,
  };
}

async function tick(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}
