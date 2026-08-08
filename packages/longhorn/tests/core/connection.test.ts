import { describe, expect, test } from "bun:test";

import {
  CheckedSnapshotConnection,
  type ConnectionFailure,
  type Unlisten,
} from "@inflatable-cookie/longhorn/core";

interface Snapshot {
  epoch: number;
  revision: number;
}

describe("CheckedSnapshotConnection", () => {
  test("registers first and closes an invalidation race with bounded refresh", async () => {
    const operations: string[] = [];
    let current = snapshot(2, 4);
    let listener: ((payload: unknown) => void) | undefined;
    let loads = 0;
    const observed: number[] = [];
    const connection = new CheckedSnapshotConnection<Snapshot>({
      async listen(next) {
        operations.push("listen");
        listener = next;
        return () => {
          operations.push("unlisten");
        };
      },
      async loadSnapshot() {
        operations.push("load");
        loads += 1;
        const result = current;
        if (loads === 1) {
          current = snapshot(2, 7);
          listener?.({ kind: "changed" });
          listener?.({ kind: "changed" });
        }
        return result;
      },
      validateSnapshot: parseSnapshot,
      handleEvent: () => ({ kind: "refresh" }),
      isNewer,
      onSnapshot(value) {
        observed.push(value.revision);
      },
    });

    expect(await connection.ready).toEqual(snapshot(2, 7));
    expect(operations.slice(0, 2)).toEqual(["listen", "load"]);
    expect(loads).toBe(2);
    expect(observed).toEqual([4, 7]);

    await connection.dispose();
    await connection.dispose();
    expect(operations.filter((operation) => operation === "unlisten")).toHaveLength(1);
  });

  test("accepts checked event snapshots and rejects stale authority", async () => {
    let listener: ((payload: unknown) => void) | undefined;
    const observed: Snapshot[] = [];
    const connection = new CheckedSnapshotConnection<Snapshot>({
      async listen(next) {
        listener = next;
        return () => {};
      },
      async loadSnapshot() {
        listener?.(snapshot(4, 9));
        return snapshot(4, 8);
      },
      validateSnapshot: parseSnapshot,
      handleEvent: (value) => ({ kind: "snapshot", value }),
      isNewer,
      onSnapshot(value) {
        observed.push(value);
      },
    });

    expect(await connection.ready).toEqual(snapshot(4, 9));
    listener?.(snapshot(3, 99));
    listener?.(snapshot(4, 9));
    expect(connection.current()).toEqual(snapshot(4, 9));
    expect(observed).toEqual([snapshot(4, 9)]);
    await connection.dispose();
  });

  test("disposes registration that resolves late without loading", async () => {
    const registration = deferred<Unlisten>();
    let loads = 0;
    let unlistens = 0;
    const connection = new CheckedSnapshotConnection<Snapshot>({
      listen: () => registration.promise,
      async loadSnapshot() {
        loads += 1;
        return snapshot(1, 0);
      },
      validateSnapshot: parseSnapshot,
      handleEvent: () => ({ kind: "refresh" }),
      isNewer,
    });

    const disposal = connection.dispose();
    registration.resolve(() => {
      unlistens += 1;
    });
    await disposal;

    expect(await connection.ready).toBeUndefined();
    expect(loads).toBe(0);
    expect(unlistens).toBe(1);
  });

  test("reports registration, snapshot, event, and unlisten failures", async () => {
    const registrationFailure = new Error("registration failed");
    const registrationFailures: ConnectionFailure[] = [];
    const failedRegistration = new CheckedSnapshotConnection<Snapshot>({
      async listen() {
        throw registrationFailure;
      },
      async loadSnapshot() {
        throw new Error("must not load");
      },
      validateSnapshot: parseSnapshot,
      handleEvent: () => ({ kind: "ignore" }),
      isNewer,
      onFailure: (failure) => registrationFailures.push(failure),
    });
    await expect(failedRegistration.ready).rejects.toBe(registrationFailure);
    expect(registrationFailures).toEqual([
      { phase: "registration", error: registrationFailure },
    ]);

    let invalidationListener: ((payload: unknown) => void) | undefined;
    let unlistens = 0;
    const eventFailures: ConnectionFailure[] = [];
    const invalidEvent = new Error("invalid event");
    const connected = new CheckedSnapshotConnection<Snapshot>({
      async listen(next) {
        invalidationListener = next;
        return () => {
          unlistens += 1;
        };
      },
      async loadSnapshot() {
        return snapshot(1, 1);
      },
      validateSnapshot: parseSnapshot,
      handleEvent() {
        throw invalidEvent;
      },
      isNewer,
      onFailure: (failure) => eventFailures.push(failure),
    });
    await connected.ready;
    invalidationListener?.({});
    await tick();
    expect(eventFailures).toEqual([
      { phase: "event", error: invalidEvent },
    ]);
    expect(unlistens).toBe(1);

    const snapshotFailure = new Error("invalid snapshot");
    let snapshotUnlistens = 0;
    const failedSnapshot = new CheckedSnapshotConnection<Snapshot>({
      async listen() {
        return () => {
          snapshotUnlistens += 1;
        };
      },
      async loadSnapshot() {
        throw snapshotFailure;
      },
      validateSnapshot: parseSnapshot,
      handleEvent: () => ({ kind: "ignore" }),
      isNewer,
    });
    await expect(failedSnapshot.ready).rejects.toBe(snapshotFailure);
    expect(failedSnapshot.failures()).toEqual([
      { phase: "snapshot", error: snapshotFailure },
    ]);
    expect(snapshotUnlistens).toBe(1);

    let validationUnlistens = 0;
    const failedValidation = new CheckedSnapshotConnection<Snapshot>({
      async listen() {
        return () => {
          validationUnlistens += 1;
        };
      },
      async loadSnapshot() {
        return { invalid: true };
      },
      validateSnapshot: parseSnapshot,
      handleEvent: () => ({ kind: "ignore" }),
      isNewer,
    });
    await expect(failedValidation.ready).rejects.toBeInstanceOf(TypeError);
    expect(
      failedValidation.failures().map(({ phase }) => phase),
    ).toEqual(["snapshot"]);
    expect(validationUnlistens).toBe(1);

    const unlistenFailure = new Error("unlisten failed");
    const failedUnlisten = new CheckedSnapshotConnection<Snapshot>({
      async listen() {
        return async () => {
          throw unlistenFailure;
        };
      },
      async loadSnapshot() {
        return snapshot(1, 1);
      },
      validateSnapshot: parseSnapshot,
      handleEvent: () => ({ kind: "ignore" }),
      isNewer,
    });
    await failedUnlisten.ready;
    await expect(failedUnlisten.dispose()).rejects.toBe(unlistenFailure);
    expect(failedUnlisten.failures()).toEqual([
      { phase: "unlisten", error: unlistenFailure },
    ]);
  });
});

function snapshot(epoch: number, revision: number): Snapshot {
  return { epoch, revision };
}

function parseSnapshot(value: unknown): Snapshot {
  if (
    typeof value !== "object" ||
    value === null ||
    !("epoch" in value) ||
    !("revision" in value) ||
    typeof value.epoch !== "number" ||
    typeof value.revision !== "number"
  ) {
    throw new TypeError("invalid snapshot");
  }
  return { epoch: value.epoch, revision: value.revision };
}

function isNewer(
  candidate: Snapshot,
  current: Snapshot | undefined,
): boolean {
  return (
    current === undefined ||
    candidate.epoch > current.epoch ||
    (candidate.epoch === current.epoch &&
      candidate.revision > current.revision)
  );
}

function deferred<Value>(): {
  promise: Promise<Value>;
  resolve(value: Value): void;
} {
  let resolve: ((value: Value) => void) | undefined;
  const promise = new Promise<Value>((accept) => {
    resolve = accept;
  });
  return {
    promise,
    resolve(value) {
      resolve?.(value);
    },
  };
}

async function tick(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}
