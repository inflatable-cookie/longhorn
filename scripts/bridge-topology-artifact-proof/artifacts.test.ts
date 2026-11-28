import { expect, test } from "bun:test";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import {
  assertBridgeSurface,
  assertProductNeutral,
  auditRustBoundary,
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

test.each([
  "longhorn-bridge/tests/contract.rs",
  "longhorn-tauri-bridge/tests/support/mod.rs",
  "longhorn-tauri-bridge/examples/permissions/query.toml",
])("audits the packed file %s outside src", async (packed) => {
  const workspace = await mkdtemp(join(tmpdir(), "longhorn-bridge-audit-"));
  const write = async (path: string, text: string): Promise<void> => {
    const target = join(workspace, "crates", path);
    await mkdir(dirname(target), { recursive: true });
    await writeFile(target, text);
  };
  try {
    await write("longhorn-bridge/src/lib.rs", "//! Neutral.");
    await write("longhorn-tauri-bridge/src/lib.rs", "//! Neutral.");
    await write(packed, 'name = "example.workspace"');
    await auditRustBoundary(workspace);

    await write(packed, 'name = "fixture.workspace"');
    await expect(auditRustBoundary(workspace)).rejects.toThrow(
      `Rust bridge artifact ${packed} contains forbidden marker: fixture.`,
    );
  } finally {
    await rm(workspace, { recursive: true, force: true });
  }
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
