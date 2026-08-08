import { expect, test } from "bun:test";
import { existsSync, readFileSync } from "node:fs";
import { realpath } from "node:fs/promises";

const packageRoot = new URL("../", import.meta.url);
const metadata = JSON.parse(
  readFileSync(new URL("package.json", packageRoot), "utf8"),
) as Record<string, unknown>;

test("package metadata keeps optional UI dependencies behind subpaths", () => {
  expect(metadata.name).toBe("@inflatable-cookie/longhorn-settings");
  expect(metadata.private).toBeTrue();
  expect(metadata.type).toBe("module");
  expect(metadata.sideEffects).toBeFalse();
  expect(metadata.scripts).toBeUndefined();
  expect(metadata.dependencies).toEqual({
    "@inflatable-cookie/longhorn-core": "0.1.0",
  });
  expect(metadata.peerDependencies).toEqual({
    "@poodle/svelte": "0.1.0",
    svelte: ">=5.38.6 <6",
  });
  expect(metadata.peerDependenciesMeta).toEqual({
    "@poodle/svelte": { optional: true },
    svelte: { optional: true },
  });

  const exports = metadata.exports as Record<
    string,
    string | Record<string, string>
  >;
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
      continue;
    }
    for (const path of Object.values(target)) {
      expect(existsSync(new URL(path, packageRoot))).toBeTrue();
    }
  }
});

test("root stays UI-free and the Poodle subpath uses public APIs only", async () => {
  const rootSource = readFileSync(
    new URL("src/index.ts", packageRoot),
    "utf8",
  );
  expect(rootSource).not.toMatch(/svelte|poodle/i);

  const poodleSource = readFileSync(
    new URL("src/poodle/SettingsShell.svelte", packageRoot),
    "utf8",
  );
  expect(poodleSource).toContain('from "@poodle/svelte"');
  expect(poodleSource).not.toMatch(
    /@poodle\/svelte\/|querySelector|poodle-(?:dialog|sidebar-nav|button)/,
  );
  expect(poodleSource).not.toMatch(
    /@longhorn\/(?:layout|surfaces|transfer|commands|history)/,
  );

  const installed = await realpath(
    new URL("../../node_modules/@poodle/svelte", packageRoot),
  );
  const poodleMetadata = JSON.parse(
    readFileSync(new URL("package.json", `file://${installed}/`), "utf8"),
  );
  expect(poodleMetadata.name).toBe("@poodle/svelte");
  expect(poodleMetadata.version).toBe("0.1.0");
});
