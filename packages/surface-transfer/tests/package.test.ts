import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(
  readFileSync(new URL("package.json", packageRoot), "utf8"),
) as Record<string, unknown>;

test("package metadata preserves the optional dependency boundary", () => {
  expect(metadata.name).toBe("@inflatable-cookie/longhorn-surface-transfer");
  expect(metadata.private).toBeTrue();
  expect(metadata.type).toBe("module");
  expect(metadata.sideEffects).toBeFalse();
  expect(metadata.scripts).toBeUndefined();
  expect(metadata.peerDependencies).toBeUndefined();
  expect(metadata.dependencies).toEqual({
    "@inflatable-cookie/longhorn-core": "0.1.0",
    "@inflatable-cookie/longhorn-surfaces": "0.1.0",
    "@inflatable-cookie/longhorn-transfer": "0.1.0",
  });

  const exports = metadata.exports as Record<
    string,
    string | Record<string, string>
  >;
  expect(Object.keys(exports)).toEqual([".", "./protocol", "./package.json"]);
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
