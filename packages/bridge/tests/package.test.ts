import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(
  readFileSync(new URL("package.json", packageRoot), "utf8"),
) as Record<string, unknown>;

test("package exports isolate optional stream and supervision support", () => {
  expect(metadata.name).toBe("@inflatable-cookie/longhorn-bridge");
  expect(metadata.private).toBeTrue();
  expect(metadata.type).toBe("module");
  expect(metadata.sideEffects).toBeFalse();
  expect(metadata.scripts).toBeUndefined();
  expect(metadata.dependencies).toEqual({
    "@inflatable-cookie/longhorn-core": "0.1.0",
  });
  // The main entry is transport-agnostic; only the ./tauri and
  // ./tauri-events subpaths need @inflatable-cookie/longhorn-tauri. Frozen in the Card 149
  // candidate receipt.
  expect(metadata.peerDependencies).toEqual({
    "@inflatable-cookie/longhorn-tauri": "0.1.0",
  });
  expect(metadata.peerDependenciesMeta).toEqual({
    "@inflatable-cookie/longhorn-tauri": { optional: true },
  });

  const exports = metadata.exports as Record<
    string,
    string | Record<string, string>
  >;
  expect(Object.keys(exports)).toEqual([
    ".",
    "./protocol",
    "./stream",
    "./supervision",
    "./tauri",
    "./tauri-events",
    "./package.json",
  ]);
  for (const target of Object.values(exports)) {
    if (typeof target === "string") {
      expect(existsSync(new URL(target, packageRoot))).toBeTrue();
      continue;
    }
    for (const path of Object.values(target)) {
      expect(existsSync(new URL(path, packageRoot))).toBeTrue();
    }
  }

  const root = readFileSync(
    new URL("src/index.ts", packageRoot),
    "utf8",
  );
  expect(root).not.toContain("./stream");
  expect(root).not.toContain("@inflatable-cookie/longhorn-core");
  expect(root).not.toContain("@tauri-apps");
  expect(root).not.toContain("svelte");
  expect(root).not.toContain("poodle");
  expect(root).not.toContain("service");
});
