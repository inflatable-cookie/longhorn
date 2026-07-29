import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";

const packagePath = new URL("../package.json", import.meta.url);
const metadata = JSON.parse(readFileSync(packagePath, "utf8"));

test("package contains only the raw host edge and exact peers", () => {
  expect(metadata.files).toEqual(["src"]);
  expect(metadata.dependencies).toEqual({
    "@longhorn/core": "0.1.0",
  });
  expect(metadata.peerDependencies).toEqual({
    "@tauri-apps/api": "^2.10.1",
  });
  expect(metadata.dependencies).not.toHaveProperty("svelte");
  expect(metadata.dependencies).not.toHaveProperty("@longhorn/layout");
  expect(metadata.dependencies).not.toHaveProperty("@longhorn/transfer");
  expect(metadata.dependencies).not.toHaveProperty("@longhorn/surfaces");
  expect(metadata.dependencies).not.toHaveProperty("@longhorn/surface-transfer");
});
