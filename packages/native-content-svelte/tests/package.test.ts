import { readFile } from "node:fs/promises";
import { resolve } from "node:path";

import { describe, expect, it } from "vitest";

const packageRoot = resolve(process.cwd(), "packages/native-content-svelte");

describe("@inflatable-cookie/longhorn-native-content-svelte package boundary", () => {
  it("has one narrow runtime dependency and no Poodle edge", async () => {
    const metadata = JSON.parse(
      await readFile(resolve(packageRoot, "package.json"), "utf8"),
    );
    expect(metadata.dependencies).toEqual({
      "@inflatable-cookie/longhorn-native-content": "0.1.0",
    });
    expect(metadata.peerDependencies).toEqual({
      svelte: ">=5.38.6 <6",
    });

    const source = await Promise.all(
      [
        "src/index.ts",
        "src/lifecycle.ts",
        "src/session.svelte.ts",
        "src/viewport.ts",
        "src/visibility.ts",
      ].map((path) => readFile(resolve(packageRoot, path), "utf8")),
    );
    const joined = source.join("\n");
    expect(joined).not.toContain("@poodle/");
    expect(joined).not.toMatch(/querySelector|closest\(|matches\(|devicePixelRatio/);
    expect(joined).not.toMatch(/keydown|pointer|mouse|semantic/);
    expect(joined).not.toMatch(/setTimeout|setInterval|requestAnimationFrame/);
  });

  it("keeps Poodle knowledge in a consumer-owned public-layout fixture", async () => {
    const fixture = await readFile(
      resolve(packageRoot, "tests/PoodleLayoutSeamHarness.svelte"),
      "utf8",
    );
    expect(fixture).toContain('from "@poodle/svelte"');
    expect(fixture).toContain("<Surface");
    expect(fixture).toContain("use:nativeContentViewport={session}");
    expect(fixture).not.toContain("@poodle/svelte/");
    expect(fixture).not.toMatch(/querySelector|closest\(|poodle-/);
  });
});
