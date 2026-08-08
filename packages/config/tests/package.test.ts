import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(
  readFileSync(new URL("package.json", packageRoot), "utf8"),
) as Record<string, unknown>;

test("framework-neutral root has one exact dependency", () => {
  expect(metadata.name).toBe("@inflatable-cookie/longhorn-config");
  expect(metadata.private).toBeTrue();
  expect(metadata.sideEffects).toBeFalse();
  expect(metadata.scripts).toBeUndefined();
  expect(metadata.dependencies).toEqual({ "@inflatable-cookie/longhorn-core": "0.1.0" });
  expect(metadata.peerDependencies).toEqual({
    "@inflatable-cookie/poodle-svelte": "0.1.0",
    svelte: ">=5.38.6 <6",
  });
  expect(metadata.peerDependenciesMeta).toEqual({
    "@inflatable-cookie/poodle-svelte": { optional: true },
    svelte: { optional: true },
  });

  const exports = metadata.exports as Record<
    string,
    string | Record<string, string>
  >;
  expect(Object.keys(exports)).toEqual([
    ".",
    "./protocol",
    "./poodle",
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
});

test("root stays framework and host free", () => {
  const root = readFileSync(new URL("src/index.ts", packageRoot), "utf8");
  const client = readFileSync(new URL("src/client.ts", packageRoot), "utf8");
  expect(`${root}\n${client}`).not.toMatch(/svelte|poodle|@tauri/i);
});

test("Poodle subpath uses only public APIs", () => {
  const poodle = [
    "src/poodle/StorageSettingsPage.svelte",
    "src/poodle/BackupSettingsPage.svelte",
  ]
    .map((path) => readFileSync(new URL(path, packageRoot), "utf8"))
    .join("\n");
  expect(poodle).toContain('from "@inflatable-cookie/poodle-svelte"');
  expect(poodle).not.toMatch(
    /@inflatable-cookie\/poodle-svelte\/|querySelector|poodle-(?:button|callout|surface|table)/,
  );
  expect(poodle).not.toMatch(/@tauri|AgeIdentity|AgePassphrase/);
});
