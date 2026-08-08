import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(readFileSync(new URL("package.json", packageRoot), "utf8")) as Record<string, unknown>;

describe("notification package boundary", () => {
  test("keeps optional renderer adapters behind subpaths", async () => {
    expect(metadata.dependencies).toEqual({ "@inflatable-cookie/longhorn-core": "0.1.0" });
    expect(metadata.peerDependenciesMeta).toEqual({ "@inflatable-cookie/poodle-svelte": { optional: true }, svelte: { optional: true } });
    const exports = metadata.exports as Record<string, string | Record<string, string>>;
    expect(Object.keys(exports)).toEqual([".", "./protocol", "./tauri", "./svelte", "./poodle", "./package.json"]);
    for (const target of Object.values(exports)) for (const path of typeof target === "string" ? [target] : Object.values(target)) expect(existsSync(new URL(path, packageRoot))).toBeTrue();
    const root = await import("../src/index.ts");
    expect(root.NotificationClient).toBeFunction();
    expect(root.NotificationController).toBeFunction();
    expect("createTauriNotificationPort" in root).toBe(false);
    expect("NotificationSession" in root).toBe(false);
    expect("NotificationPanel" in root).toBe(false);
  });

  test("root has no operation, command, Tauri, Svelte, or Poodle edge", () => {
    const sources = ["src/index.ts", "src/client.ts", "src/controller.ts", "src/direct.ts", "src/serialized.ts"].map((path) => readFileSync(new URL(path, packageRoot), "utf8")).join("\n");
    for (const forbidden of ["@inflatable-cookie/longhorn-operation", "@inflatable-cookie/longhorn-command", "@tauri-apps", 'from "svelte', "@inflatable-cookie/poodle-", "./tauri", "./svelte", "./poodle"]) expect(sources).not.toContain(forbidden);
  });

  test("Poodle adapters use both public toast exports", () => {
    const sources = ["src/poodle/NotificationToastHost.svelte", "src/poodle/NotificationToastStack.svelte", "src/poodle/NotificationPanel.svelte"].map((path) => readFileSync(new URL(path, packageRoot), "utf8")).join("\n");
    expect(sources).toContain("ToastHost");
    expect(sources).toContain("ToastStack");
    expect(sources).toContain('from "@inflatable-cookie/poodle-svelte"');
    expect(sources).not.toContain("@inflatable-cookie/poodle-svelte/");
    expect(sources).not.toContain("/components/src/");
  });
});
