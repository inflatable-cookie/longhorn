import { readFile, realpath } from "node:fs/promises";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const packageRoot = resolve(process.cwd(), "packages/poodle");
const repositoryRoot = resolve(packageRoot, "../..");

describe("@longhorn/poodle package boundary", () => {
  it("pins exact public peers and keeps the root Surface-free", async () => {
    const metadata = JSON.parse(
      await readFile(resolve(packageRoot, "package.json"), "utf8"),
    );
    expect(metadata.dependencies).toEqual({
      "@longhorn/layout": "0.1.0",
      "@longhorn/svelte": "0.1.0",
    });
    expect(metadata.peerDependencies).toEqual({
      "@longhorn/transfer": "0.1.0",
      "@poodle/svelte": "0.1.0",
      svelte: ">=5.38.6 <6",
    });
    expect(metadata.peerDependenciesMeta).toEqual({
      "@longhorn/transfer": { optional: true },
    });

    const source = (
      await Promise.all(
        [
          "src/index.ts",
          "src/types.ts",
          "src/binding.ts",
          "src/document.ts",
          "src/projectors.ts",
          "src/LayoutTabs.svelte",
          "src/LayoutDockRegion.svelte",
          "src/LayoutSplitView.svelte",
        ].map((path) => readFile(resolve(packageRoot, path), "utf8")),
      )
    ).join("\n");
    expect(source).not.toMatch(
      /@longhorn\/(?:surfaces|transfer|surface-transfer)/,
    );
    expect(source).not.toMatch(
      /poodle-(?:tabs|dock-region|split-view)|application\/x-poodle|querySelector/,
    );
    expect(source).not.toContain("/Users/");

    const transferSource = await readFile(
      resolve(packageRoot, "src/transfer.ts"),
      "utf8",
    );
    expect(transferSource).toContain('from "@longhorn/transfer"');
    expect(transferSource).not.toMatch(
      /poodle-(?:tabs|dock-region|split-view)|application\/x-poodle|querySelector/,
    );
  });

  it("resolves the exact packed Poodle package, not sibling source", async () => {
    const installedRoot = await realpath(
      resolve(repositoryRoot, "node_modules/@poodle/svelte"),
    );
    const donorSource = await realpath(
      resolve(repositoryRoot, "../poodle/packages/svelte/components"),
    );
    const metadata = JSON.parse(
      await readFile(resolve(installedRoot, "package.json"), "utf8"),
    );

    expect(installedRoot).not.toBe(donorSource);
    expect(installedRoot.startsWith(`${donorSource}/`)).toBe(false);
    expect(metadata.name).toBe("@poodle/svelte");
    expect(metadata.version).toBe("0.1.0");
  });
});
