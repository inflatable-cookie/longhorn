import { expect, test } from "bun:test";
import type { EventTransport, Unlisten } from "@inflatable-cookie/longhorn-core";

import {
  TRANSFER_CLIENT_CHANGED_EVENT,
  TransferClient,
  TransferClientConnectionDisposedError,
  type TransferClientSnapshot,
} from "@inflatable-cookie/longhorn-transfer";

test("installs the epoch listener before snapshot and keeps the newest authority", async () => {
  const operations: string[] = [];
  let listener: ((payload: unknown) => void) | undefined;
  const response = snapshot("client:response", 1);
  const event = snapshot("client:event", 2);
  const transport: EventTransport = {
    async listen(name, next) {
      operations.push(`listen:${name}`);
      listener = next;
      return () => {
        operations.push("unlisten");
      };
    },
    async invoke(command) {
      operations.push(`invoke:${command}`);
      listener?.(event);
      return response;
    },
  };
  const observed: TransferClientSnapshot[] = [];
  const connection = new TransferClient(transport).connect((value) => {
    observed.push(value);
  });

  expect(await connection.ready).toEqual(event);
  expect(connection.current()).toEqual(event);
  expect(observed).toEqual([event]);
  expect(operations).toEqual([
    `listen:${TRANSFER_CLIENT_CHANGED_EVENT}`,
    "invoke:longhorn_transfer_snapshot",
  ]);

  await connection.dispose();
  expect(operations.at(-1)).toBe("unlisten");
});

test("late listener registration tears down without invoking snapshot", async () => {
  let resolveListen:
    | ((unlisten: Unlisten) => void)
    | undefined;
  let invokeCount = 0;
  let unlistenCount = 0;
  const transport: EventTransport = {
    listen() {
      return new Promise((resolve) => {
        resolveListen = resolve;
      });
    },
    async invoke() {
      invokeCount += 1;
      return snapshot("client:unused", 1);
    },
  };
  const connection = new TransferClient(transport).connect();
  const disposal = connection.dispose();
  resolveListen?.(() => {
    unlistenCount += 1;
  });

  await expect(connection.ready).rejects.toBeInstanceOf(
    TransferClientConnectionDisposedError,
  );
  await disposal;
  expect(invokeCount).toBe(0);
  expect(unlistenCount).toBe(1);
});

test("ignores stale client epochs and reports invalid events", async () => {
  let listener: ((payload: unknown) => void) | undefined;
  let unlistenCount = 0;
  const failures: string[] = [];
  const observed: TransferClientSnapshot[] = [];
  const transport: EventTransport = {
    async listen(_name, next) {
      listener = next;
      return () => {
        unlistenCount += 1;
      };
    },
    async invoke() {
      return snapshot("client:current", 4);
    },
  };
  const connection = new TransferClient(transport).connect(
    (value) => observed.push(value),
    ({ phase }) => failures.push(phase),
  );
  await connection.ready;

  listener?.(snapshot("client:stale", 3));
  expect(connection.current()).toEqual(snapshot("client:current", 4));
  expect(observed).toEqual([snapshot("client:current", 4)]);

  listener?.({ protocol_version: 99 });
  await tick();
  expect(failures).toEqual(["event"]);
  expect(connection.failures().map(({ phase }) => phase)).toEqual([
    "event",
  ]);
  expect(unlistenCount).toBe(1);
});

function snapshot(
  clientId: string,
  epoch: number,
): TransferClientSnapshot {
  return {
    protocol_version: 1,
    client_id: clientId,
    client_epoch: epoch,
    current_lease_generation: null,
  };
}

async function tick(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}
