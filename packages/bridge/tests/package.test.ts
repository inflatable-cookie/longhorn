import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(
  readFileSync(new URL("package.json", packageRoot), "utf8"),
) as Record<string, unknown>;

test("package exports isolate optional stream and supervision support", () => {
  expect(metadata.name).toBe("@longhorn/bridge");
  expect(metadata.private).toBeTrue();
  expect(metadata.type).toBe("module");
  expect(metadata.sideEffects).toBeFalse();
  expect(metadata.scripts).toBeUndefined();
  expect(metadata.peerDependencies).toBeUndefined();
  // The main entry is transport-agnostic, but the hard @longhorn/tauri
  // dependency is part of the frozen Card 127 candidate receipt; demoting
  // it to an optional peer is deferred to the next distribution candidate.
  expect(metadata.dependencies).toEqual({
    "@longhorn/core": "0.1.0",
    "@longhorn/tauri": "0.1.0",
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
  expect(root).not.toContain("@longhorn/core");
  expect(root).not.toContain("@tauri-apps");
  expect(root).not.toContain("svelte");
  expect(root).not.toContain("poodle");
  expect(root).not.toContain("service");
});
