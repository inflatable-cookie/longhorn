import { expect, test } from "bun:test";
import {
  assertBridgeSurface,
  assertProductNeutral,
  bridgePackageName,
  packedTypescriptPackageNames,
} from "./artifacts.ts";

test("allows prose and longer identifiers that mention network, fixture and credential words", () => {
  const source = [
    "// Subscribe to the event source and reuse the web socket's connection.",
    "// We fetch the data from a local fixture.",
    "const eventSourceId = currentSubscription();",
    "const $EventSource = bridgeName;",
    "const api_key_present = false;",
    "prefetch (page);",
  ].join("\n");
  expect(() => assertProductNeutral("test artifact", source)).not.toThrow();
});

test.each([
  ["const reader: EventSource = source", "eventsource"],
  ["new EventSource(url)", "eventsource"],
  ["import { read } from 'eventsource'", "eventsource"],
  ["new WebSocket (url)", "websocket"],
  ["client.fetch \n (url)", "fetch("],
  ["use reqwest::Client;", "reqwest"],
  ["tokio-tungstenite = '*'", "tokio-tungstenite"],
  ["const token = credentials.access_token", "access_token"],
  ["const name = 'ordered-streams';", "ordered-streams"],
  ["const domain = 'fixture.catalog';", "fixture."],
])("rejects %s as the %s marker", (source, marker) => {
  expect(() => assertProductNeutral("test artifact", source)).toThrow(
    `test artifact contains forbidden marker: ${marker}`,
  );
});

const bridgeExportMap = {
  ".": "./src/core/index.ts",
  "./bridge": "./src/bridge/index.ts",
  "./bridge/protocol": "./src/bridge/protocol.ts",
  "./bridge/stream": "./src/bridge/stream.ts",
  "./bridge/supervision": "./src/bridge/supervision.ts",
  "./core": "./src/core/index.ts",
};
const protocolSource = "export const BRIDGE_PROTOCOL_VERSION = 1 as const;";

test("the bridge surface check runs on a packed package that exists", () => {
  expect(packedTypescriptPackageNames).toContain(bridgePackageName);
  expect(bridgePackageName).toBe("@inflatable-cookie/longhorn");
});

test("the packed bridge exports must be exactly the declared ./bridge set", () => {
  expect(() =>
    assertBridgeSurface(bridgePackageName, bridgeExportMap, protocolSource),
  ).not.toThrow();
  const { "./bridge/stream": _removed, ...missingStream } = bridgeExportMap;
  expect(() => assertBridgeSurface(bridgePackageName, missingStream, protocolSource)).toThrow(
    "bridge exports mismatch",
  );
  expect(() =>
    assertBridgeSurface(
      bridgePackageName,
      { ...bridgeExportMap, "./bridge/internal": "./src/bridge/internal.ts" },
      protocolSource,
    ),
  ).toThrow("bridge exports mismatch");
});

test("the packed source must carry the bridge protocol version", () => {
  expect(() => assertBridgeSurface(bridgePackageName, bridgeExportMap, "export {};")).toThrow(
    "packed bridge protocol version is missing",
  );
});
