import { describe, expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(
  readFileSync(new URL("package.json", packageRoot), "utf8"),
) as Record<string, unknown>;

describe("native-content package boundary", () => {
  test("root is framework-neutral and optional Tauri assembly is isolated", async () => {
    expect(metadata.name).toBe("@inflatable-cookie/longhorn-native-content");
    expect(metadata.dependencies).toEqual({ "@inflatable-cookie/longhorn-core": "0.1.0" });
    expect("peerDependencies" in metadata).toBeFalse();
    const exports = metadata.exports as Record<
      string,
      string | Record<string, string>
    >;
    expect(Object.keys(exports)).toEqual([
      ".",
      "./protocol",
      "./tauri",
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
    expect(root.NativeContentClient).toBeFunction();
    expect(root.SerializedNativeContentPort).toBeFunction();
    expect("createTauriNativeContentPort" in root).toBeFalse();
  });

  test("minimal manifest has no UI, browser, plugin, or GPU dependency", () => {
    const manifest = readFileSync(new URL("package.json", packageRoot), "utf8");
    for (const forbidden of [
      "svelte",
      "poodle",
      "@tauri-apps",
      "playwright",
      "webview",
      "wgpu",
      "vst",
    ]) {
      expect(manifest.toLowerCase()).not.toContain(forbidden);
    }
  });

  test("capabilities admit protocol commands only", () => {
    const readPermission = readFileSync(
      new URL("examples/permissions/read-native-content.toml", packageRoot),
      "utf8",
    );
    const mutatePermission = readFileSync(
      new URL("examples/permissions/mutate-native-content.toml", packageRoot),
      "utf8",
    );
    expect(readPermission).toContain("longhorn_native_content_connect");
    expect(readPermission).toContain("longhorn_native_content_snapshot");
    expect(mutatePermission).toContain("longhorn_native_content_update_desired");
    expect(mutatePermission).toContain("longhorn_native_content_decide_size");
    const combined = `${readPermission}\n${mutatePermission}`.toLowerCase();
    for (const forbidden of [
      "navigate",
      "download",
      "plugin",
      "render",
      "product",
      "remote",
    ]) {
      expect(combined).not.toContain(forbidden);
    }
  });
});
