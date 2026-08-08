import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { join } from "node:path";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(
  readFileSync(new URL("package.json", packageRoot), "utf8"),
) as Record<string, unknown>;

const DOMAINS = [
  "bridge",
  "commands",
  "config",
  "core",
  "history",
  "history-tree",
  "layout",
  "native-content",
  "notifications",
  "operation",
  "settings",
  "surface-transfer",
  "surfaces",
  "transfer",
];

function sourceFiles(dir: string): string[] {
  return readdirSync(dir).flatMap((entry) => {
    const path = join(dir, entry);
    if (statSync(path).isDirectory()) return sourceFiles(path);
    return /\.(ts|svelte)$/.test(entry) ? [path] : [];
  });
}

describe("@inflatable-cookie/longhorn package boundary", () => {
  test("declares no dependencies and no peers", () => {
    expect(metadata.dependencies).toBeUndefined();
    expect(metadata.peerDependencies).toBeUndefined();
    expect(metadata.peerDependenciesMeta).toBeUndefined();
  });

  test("exports one root per domain and every target resolves", () => {
    const exports = metadata.exports as Record<
      string,
      string | Record<string, string>
    >;
    for (const domain of DOMAINS) {
      expect(exports[`./${domain}`]).toBeDefined();
    }
    for (const target of Object.values(exports)) {
      const paths = typeof target === "string" ? [target] : Object.values(target);
      for (const path of paths) {
        expect(existsSync(new URL(path, packageRoot))).toBeTrue();
      }
    }
  });

  // The peerless tier is the whole point of the consolidation: if any file
  // here reaches a host or a UI framework, the package cannot stay peerless.
  test("no source file carries a host, framework, or UI edge", () => {
    const forbidden = [
      "@tauri-apps",
      'from "svelte"',
      "@inflatable-cookie/poodle-",
      "@inflatable-cookie/longhorn-poodle-svelte",
      "@inflatable-cookie/longhorn-tauri",
    ];
    const offenders = sourceFiles(new URL("src", packageRoot).pathname).filter(
      (file) => {
        const text = readFileSync(file, "utf8");
        return forbidden.some((needle) => text.includes(needle));
      },
    );
    expect(offenders).toEqual([]);
  });

  test("domain roots expose their clients and hide their adapters", async () => {
    const table: Record<string, { present: string[]; absent: string[] }> = {
      "native-content": {
        present: ["NativeContentClient", "SerializedNativeContentPort"],
        absent: [],
      },
      notifications: {
        present: ["NotificationClient", "NotificationController"],
        absent: [
          "createTauriNotificationPort",
          "NotificationSession",
          "NotificationPanel",
        ],
      },
      operation: {
        present: ["OperationClient", "OperationController"],
        absent: [
          "createTauriOperationPort",
          "createBridgeOperationPort",
          "OperationSession",
          "OperationPanel",
        ],
      },
    };
    for (const [domain, { present, absent }] of Object.entries(table)) {
      const root = (await import(`../src/${domain}/index.ts`)) as Record<
        string,
        unknown
      >;
      for (const symbol of present) expect(root[symbol]).toBeFunction();
      for (const symbol of absent) expect(symbol in root).toBe(false);
    }
  });
});
