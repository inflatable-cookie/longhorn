import { expect, mock, test } from "bun:test";
import type { Options as EventListenOptions } from "@tauri-apps/api/event";
import { TRANSFER_CLIENT_CHANGED_EVENT } from "@inflatable-cookie/longhorn/transfer";

const invokes: Array<{
  command: string;
  arguments_: Record<string, unknown>;
}> = [];
const listens: Array<{ event: string; target: EventListenOptions["target"] }> = [];
let eventListener: ((event: { payload: unknown }) => void) | undefined;
let unlistenCount = 0;

await mock.module("@tauri-apps/api/core", () => ({
  invoke: async (command: string, arguments_: Record<string, unknown>) => {
    invokes.push({ command, arguments_ });
    return { ok: true };
  },
}));
await mock.module("@tauri-apps/api/event", () => ({
  listen: async (
    event: string,
    listener: (event: { payload: unknown }) => void,
    options?: EventListenOptions,
  ) => {
    listens.push({ event, target: options?.target });
    eventListener = listener;
    return () => {
      unlistenCount += 1;
    };
  },
}));
await mock.module("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({ label: "current-view" }),
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
  expect(listens).toEqual([{ event: "event", target: undefined }]);
  expect(payloads).toEqual([{ epoch: 2 }]);
  expect(unlistenCount).toBe(1);
});

test("subscribes to transfer client changes for the current webview", async () => {
  const { TauriEventTransport } = await import("@inflatable-cookie/longhorn-tauri/events");
  const transport = new TauriEventTransport();
  const unlisten = await transport.listen(TRANSFER_CLIENT_CHANGED_EVENT, () => {});
  await unlisten();

  expect(listens.at(-1)).toEqual({
    event: TRANSFER_CLIENT_CHANGED_EVENT,
    target: "current-view",
  });
});
