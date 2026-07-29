import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const packageRoot = resolve(process.cwd(), "packages/svelte");

describe("@longhorn/svelte package boundary", () => {
  it("keeps the root Surface-free and marks domain peers optional", async () => {
    const metadata = JSON.parse(
      await readFile(resolve(packageRoot, "package.json"), "utf8"),
    );
    expect(metadata.dependencies).toEqual({
      "@longhorn/core": "0.1.0",
    });
    expect(metadata.peerDependencies.svelte).toBe(
      ">=5.38.6 <=5.56.8",
    );
    for (const peer of [
      "@longhorn/layout",
      "@longhorn/surfaces",
      "@longhorn/transfer",
      "@longhorn/surface-transfer",
    ]) {
      expect(metadata.peerDependenciesMeta[peer]).toEqual({
        optional: true,
      });
    }

    const rootSource = await Promise.all(
      ["src/index.ts", "src/lifecycle.svelte.ts", "src/optimistic.svelte.ts"]
        .map((path) => readFile(resolve(packageRoot, path), "utf8")),
    );
    expect(rootSource.join("\n")).not.toMatch(
      /@longhorn\/(?:layout|surfaces|transfer|surface-transfer)/,
    );
  });

  it("makes optional subpath failures name their missing peer", async () => {
    const surfaceSource = await readFile(
      resolve(packageRoot, "src/surfaces.svelte.ts"),
      "utf8",
    );
    const surfaceTransferSource = await readFile(
      resolve(packageRoot, "src/surface-transfer.svelte.ts"),
      "utf8",
    );
    expect(surfaceSource).toContain('from "@longhorn/surfaces"');
    expect(surfaceTransferSource).toContain(
      'from "@longhorn/surface-transfer"',
    );
  });
});
