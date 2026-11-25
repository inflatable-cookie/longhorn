import { afterEach, beforeEach, expect, spyOn, test } from "bun:test";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC, mockWindows } from "@tauri-apps/api/mocks";

import { TauriHostError, TauriTransport } from "@inflatable-cookie/longhorn-tauri";
import {
  BRIDGE_DOMAIN_EVENT,
  BRIDGE_PROGRESS_EVENT,
  BRIDGE_TERMINAL_EVENT,
  listenTauriBridgeJob,
  TauriBridgeStreamSource,
} from "@inflatable-cookie/longhorn-tauri/bridge-events";
import { TauriEventTransport } from "@inflatable-cookie/longhorn-tauri/events";
import {
  TransferClient,
  TRANSFER_CLIENT_CHANGED_EVENT,
} from "@inflatable-cookie/longhorn/transfer";

import { jsonCodec } from "../../../longhorn/tests/bridge/support.ts";
import bridgeErrors from "../../../../fixtures/bridge/host-errors.json";
import commandErrors from "../../../../fixtures/commands/host-errors.json";
import configErrors from "../../../../fixtures/config/host-errors.json";
import historyErrors from "../../../../fixtures/history/host-errors.json";
import historyTreeErrors from "../../../../fixtures/history-tree/host-errors.json";
import licenceErrors from "../../../../fixtures/licence/host-errors.json";
import notificationErrors from "../../../../fixtures/notifications/host-errors.json";
import operationErrors from "../../../../fixtures/operation/host-errors.json";
import settingsErrors from "../../../../fixtures/settings/host-errors.json";
import transferErrors from "../../../../fixtures/transfer/host-errors.json";
import transferProtocol from "../../../../fixtures/transfer/protocol-v1.json";
import updateErrors from "../../../../fixtures/update/host-errors.json";

const originalWindow = Object.getOwnPropertyDescriptor(globalThis, "window");

beforeEach(() => {
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: Object.assign(new EventTarget(), { crypto: globalThis.crypto }),
  });
});

afterEach(() => {
  clearMocks();
  if (originalWindow === undefined) {
    Reflect.deleteProperty(globalThis, "window");
  } else {
    Object.defineProperty(globalThis, "window", originalWindow);
  }
});

test("adapts raw Tauri invoke without event support", async () => {
  mockIPC((command, args) => ({ command, args }));
  const transport = new TauriTransport();

  expect(await transport.invoke("command", { request: 1 })).toEqual({
    command: "command",
    args: { request: 1 },
  });
  expect("listen" in transport).toBeFalse();
});

test("turns Rust host errors into Error instances with the original fields", async () => {
  const transport = new TauriTransport();
  for (const error of [
    ...bridgeErrors,
    ...commandErrors,
    ...configErrors,
    ...historyErrors,
    ...historyTreeErrors,
    ...licenceErrors,
    ...notificationErrors,
    ...operationErrors,
    ...settingsErrors,
    ...transferErrors,
    ...updateErrors,
  ]) {
    mockIPC(() => {
      throw error;
    });
    const rejection = await transport.invoke("command", {}).catch((error_: unknown) => error_);
    expect(rejection).toBeInstanceOf(TauriHostError);
    expect(rejection).toMatchObject(error);
  }
});

test("preserves Tauri and malformed rejections", async () => {
  const transport = new TauriTransport();
  for (const error of [
    "command not found",
    new Error("IPC disconnected"),
    { code: "test", message: "bad type", retryable: "yes" },
    { code: "test", message: "unknown field", retryable: false, extra: true },
  ]) {
    mockIPC(() => {
      throw error;
    });
    const rejection = await transport.invoke("command", {}).catch((error_: unknown) => error_);
    expect(rejection).toBe(error);
  }
});

test("adapts broadcast event payloads until unlisten", async () => {
  mockIPC(() => {}, { shouldMockEvents: true });
  const transport = new TauriEventTransport();
  const payloads: Array<unknown> = [];
  const unlisten = await transport.listen("event", (payload) => payloads.push(payload));
  await emit("event", { epoch: 2 });
  await unlisten();
  expect(payloads).toEqual([{ epoch: 2 }]);
});

test("the Transfer client subscribes to change hints for the current webview", async () => {
  mockWindows("current-view");
  const registrations: Array<unknown> = [];
  mockIPC((command, args) => {
    if (command === "plugin:event|listen") {
      registrations.push(args);
      return 1;
    }
    return transferProtocol.client_snapshot;
  });

  const connection = new TransferClient(new TauriEventTransport()).connect();
  try {
    await expect(connection.ready).resolves.toEqual(transferProtocol.client_snapshot);
    expect(registrations).toEqual([
      expect.objectContaining({
        event: TRANSFER_CLIENT_CHANGED_EVENT,
        target: { kind: "WebviewWindow", label: "current-view" },
      }),
    ]);
  } finally {
    await connection.dispose();
  }
});

test("all three bridge listeners register and clean up inside their own renderer", async () => {
  const registrations: Array<string> = [];
  mockIPC((command) => {
    registrations.push(command);
    return null;
  });
  const add = spyOn(window, "addEventListener");
  const remove = spyOn(window, "removeEventListener");
  const payloads: Array<unknown> = [];
  const source = new TauriBridgeStreamSource("session:test", "example.workspace");
  const unlisten = await source.listen((payload) => payloads.push(payload));
  const stopJob = await listenTauriBridgeJob("request:test", "job:test", {
    progressCodec: jsonCodec,
    successCodec: jsonCodec,
    detailCodec: jsonCodec,
    terminal: () => {},
  });

  const events = [BRIDGE_DOMAIN_EVENT, BRIDGE_PROGRESS_EVENT, BRIDGE_TERMINAL_EVENT];
  expect(add.mock.calls.map(([event]) => event)).toEqual(events);
  const otherRenderer = new EventTarget();
  otherRenderer.dispatchEvent(new CustomEvent(BRIDGE_DOMAIN_EVENT, { detail: "other" }));
  window.dispatchEvent(new CustomEvent(BRIDGE_DOMAIN_EVENT, { detail: "own" }));
  await unlisten();
  await stopJob();
  window.dispatchEvent(new CustomEvent(BRIDGE_DOMAIN_EVENT, { detail: "after cleanup" }));
  expect(payloads).toEqual(["own"]);
  expect(remove.mock.calls.map(([event]) => event)).toEqual([
    BRIDGE_DOMAIN_EVENT,
    BRIDGE_TERMINAL_EVENT,
    BRIDGE_PROGRESS_EVENT,
  ]);
  expect(registrations).toEqual([]);
});
