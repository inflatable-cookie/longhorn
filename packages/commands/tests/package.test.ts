import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(
  readFileSync(new URL("package.json", packageRoot), "utf8"),
) as Record<string, unknown>;

test("package exports root, protocol, and optional adapter surfaces", () => {
  expect(metadata.name).toBe("@inflatable-cookie/longhorn-commands");
  expect(metadata.private).toBeTrue();
  expect(metadata.sideEffects).toBeFalse();
  expect(metadata.scripts).toBeUndefined();
  expect(metadata.dependencies).toBeUndefined();
  const exports = metadata.exports as Record<string, string | Record<string, string>>;
  expect(Object.keys(exports)).toEqual([
    ".",
    "./protocol",
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
