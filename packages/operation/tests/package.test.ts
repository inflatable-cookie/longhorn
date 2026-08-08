import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(
  readFileSync(new URL("package.json", packageRoot), "utf8"),
) as Record<string, unknown>;

describe("operation package boundary", () => {
  test("keeps every optional adapter behind a declared subpath", async () => {
    expect(metadata.name).toBe("@inflatable-cookie/longhorn-operation");
    expect(metadata.dependencies).toEqual({ "@inflatable-cookie/longhorn-core": "0.1.0" });
    expect(metadata.peerDependencies).toEqual({
      "@inflatable-cookie/longhorn-bridge": "0.1.0",
      "@poodle/svelte": "0.1.0",
      svelte: ">=5.38.6 <6",
    });
    expect(metadata.peerDependenciesMeta).toEqual({
      "@inflatable-cookie/longhorn-bridge": { optional: true },
      "@poodle/svelte": { optional: true },
      svelte: { optional: true },
    });

    const exports = metadata.exports as Record<
      string,
      string | Record<string, string>
    >;
    expect(Object.keys(exports)).toEqual([
      ".",
      "./protocol",
      "./tauri",
      "./svelte",
      "./poodle",
      "./bridge",
      "./package.json",
    ]);
    for (const target of Object.values(exports)) {
      if (typeof target === "string") {
        expect(existsSync(new URL(target, packageRoot))).toBeTrue();
      } else {
        for (const path of Object.values(target)) {
          expect(existsSync(new URL(path, packageRoot))).toBeTrue();
        }
      }
    }

    const root = await import("../src/index.ts");
    expect(root.OperationClient).toBeFunction();
    expect(root.OperationController).toBeFunction();
    expect("createTauriOperationPort" in root).toBe(false);
    expect("createBridgeOperationPort" in root).toBe(false);
    expect("OperationSession" in root).toBe(false);
    expect("OperationPanel" in root).toBe(false);
  });

  test("root and Svelte sources do not resolve Poodle", () => {
    const root = readFileSync(new URL("src/index.ts", packageRoot), "utf8");
    const controller = readFileSync(
      new URL("src/controller.ts", packageRoot),
      "utf8",
    );
    const svelte = [
      "src/svelte.ts",
      "src/svelte/session.svelte.ts",
      "src/svelte/lifecycle.ts",
    ]
      .map((path) => readFileSync(new URL(path, packageRoot), "utf8"))
      .join("\n");
    for (const source of [root, controller, svelte]) {
      expect(source).not.toContain("@poodle/");
      expect(source).not.toContain("./poodle");
    }
  });

  test("Poodle projection imports only the public package surface", () => {
    const panel = readFileSync(
      new URL("src/poodle/OperationPanel.svelte", packageRoot),
      "utf8",
    );
    expect(panel).toContain('from "@poodle/svelte"');
    expect(panel).not.toContain("@poodle/svelte/");
    expect(panel).not.toContain("@poodle/styles");
    expect(panel).not.toContain("/components/src/");
    for (const name of [
      "Progress",
      "StatusIndicator",
      "InlineListSection",
      "Button",
      "AlertDialog",
    ]) {
      expect(panel).toContain(name);
    }
  });
});
