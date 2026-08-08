import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(
  readFileSync(new URL("package.json", packageRoot), "utf8"),
) as Record<string, unknown>;

test("package contains only pure framework-neutral source", () => {
  expect(metadata.name).toBe("@inflatable-cookie/longhorn-core");
  expect(metadata.private).toBeTrue();
  expect(metadata.type).toBe("module");
  expect(metadata.sideEffects).toBeFalse();
  expect(metadata.scripts).toBeUndefined();
  expect(metadata.dependencies).toBeUndefined();
  expect(metadata.peerDependencies).toBeUndefined();
  expect(metadata.files).toEqual(["src"]);

  const exports = metadata.exports as Record<
    string,
    string | Record<string, string>
  >;
  expect(Object.keys(exports)).toEqual([".", "./package.json"]);
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
