import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(
  readFileSync(new URL("package.json", packageRoot), "utf8"),
) as Record<string, unknown>;

test("package exposes optional Tauri, Svelte, and Poodle subpaths", () => {
  expect(metadata.name).toBe("@inflatable-cookie/longhorn-history");
  expect(metadata.private).toBeTrue();
  expect(metadata.sideEffects).toBeFalse();
  expect(metadata.scripts).toBeUndefined();
  expect(metadata.dependencies).toEqual({ "@inflatable-cookie/longhorn-core": "0.1.0" });
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
    "./package.json",
  ]);
  for (const target of Object.values(exports)) {
    if (typeof target === "string") {
      expect(existsSync(new URL(target, packageRoot))).toBeTrue();
    } else {
      Object.values(target).forEach((path) =>
        expect(existsSync(new URL(path, packageRoot))).toBeTrue(),
      );
    }
  }
});

test("root source has no optional adapter import", () => {
  const root = readFileSync(new URL("src/index.ts", packageRoot), "utf8");
  for (const forbidden of ["./tauri", "./svelte", "./poodle", "@poodle", "svelte"]) {
    expect(root).not.toContain(forbidden);
  }
});
