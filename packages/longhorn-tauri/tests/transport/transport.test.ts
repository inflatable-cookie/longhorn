import { expect, mock, test } from "bun:test";

const invokes: Array<{
  command: string;
  arguments_: Record<string, unknown>;
}> = [];
const listens: Array<string> = [];
let eventListener: ((event: { payload: unknown }) => void) | undefined;
let unlistenCount = 0;

await mock.module("@tauri-apps/api/core", () => ({
  invoke: async (command: string, arguments_: Record<string, unknown>) => {
    invokes.push({ command, arguments_ });
    return { ok: true };
  },
}));
await mock.module("@tauri-apps/api/event", () => ({
  listen: async (event: string, listener: (event: { payload: unknown }) => void) => {
    listens.push(event);
    eventListener = listener;
    return () => {
      unlistenCount += 1;
    };
  },
}));

test("adapts raw Tauri invoke without importing event support", async () => {
  const { TauriTransport } = await import("@inflatable-cookie/longhorn-tauri");
  const transport = new TauriTransport();

  expect(await transport.invoke("command", { request: 1 })).toEqual({
    ok: true,
  });
  expect("listen" in transport).toBeFalse();
  expect(listens).toEqual([]);
});

test("adapts optional raw Tauri event payloads once", async () => {
  const { TauriEventTransport } = await import("@inflatable-cookie/longhorn-tauri/events");
  const transport = new TauriEventTransport();
  const payloads: Array<unknown> = [];
  const unlisten = await transport.listen("event", (payload) => {
    payloads.push(payload);
  });
  eventListener?.({ payload: { epoch: 2 } });
  await unlisten();

  expect(invokes).toEqual([{ command: "command", arguments_: { request: 1 } }]);
  expect(listens).toEqual(["event"]);
  expect(payloads).toEqual([{ epoch: 2 }]);
  expect(unlistenCount).toBe(1);
});
