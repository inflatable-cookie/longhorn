import { readFile, readdir, stat } from "node:fs/promises";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { describe, expect, it } from "vitest";

const packageRoot = resolve(process.cwd(), "packages/longhorn-poodle-svelte");
const repositoryRoot = resolve(packageRoot, "../..");

const POODLE = "@inflatable-cookie/poodle-svelte";

type Manifest = {
  dependencies?: Record<string, string>;
  devDependencies?: Record<string, string>;
  peerDependencies?: Record<string, string>;
};

function manifest(root: string): Manifest {
  return JSON.parse(
    readFileSync(resolve(root, "package.json"), "utf8"),
  ) as Manifest;
}

const rootManifest = manifest(repositoryRoot);
const packageManifest = manifest(packageRoot);

async function sourceFiles(dir: string): Promise<string[]> {
  const entries = await readdir(dir);
  const nested = await Promise.all(
    entries.map(async (entry) => {
      const path = join(dir, entry);
      if ((await stat(path)).isDirectory()) return sourceFiles(path);
      return /\.(ts|svelte)$/.test(entry) ? [path] : [];
    }),
  );
  return nested.flat();
}

describe("@inflatable-cookie/longhorn-poodle-svelte package boundary", () => {
  it("pins exact public peers and keeps Poodle optional", async () => {
    const metadata = JSON.parse(
      await readFile(resolve(packageRoot, "package.json"), "utf8"),
    );
    expect(metadata.dependencies).toBeUndefined();
    expect(metadata.peerDependencies).toEqual({
      "@inflatable-cookie/longhorn": "0.1.0",
      "@inflatable-cookie/poodle-svelte": "0.1.0",
      svelte: ">=5.38.6 <=5.56.8",
    });
    expect(metadata.peerDependenciesMeta).toEqual({
      "@inflatable-cookie/poodle-svelte": { optional: true },
    });
  });

  it("resolves every export target", async () => {
    const metadata = JSON.parse(
      await readFile(resolve(packageRoot, "package.json"), "utf8"),
    );
    const exports = metadata.exports as Record<
      string,
      string | Record<string, string>
    >;
    for (const target of Object.values(exports)) {
      const paths =
        typeof target === "string" ? [target] : Object.values(target);
      for (const path of paths) {
        await expect(stat(resolve(packageRoot, path))).resolves.toBeDefined();
      }
    }
  });

  // The projection tier consumes the framework, but only across its published
  // surface. A relative path into the sibling package would defeat the split.
  it("reaches the framework only through its public specifiers", async () => {
    const files = await sourceFiles(resolve(packageRoot, "src"));
    const offenders: string[] = [];
    for (const file of files) {
      const text = await readFile(file, "utf8");
      if (/from "\.\.\/\.\.\/longhorn\//.test(text)) offenders.push(file);
    }
    expect(offenders).toEqual([]);
  });

  // Contract 012: Poodle is consumed as one public package, never by reaching
  // past its root into component source.
  it("consumes Poodle only at its package root", async () => {
    const files = await sourceFiles(resolve(packageRoot, "src"));
    const offenders: string[] = [];
    for (const file of files) {
      const text = await readFile(file, "utf8");
      if (
        text.includes("@inflatable-cookie/poodle-svelte/") ||
        text.includes("/components/src/") ||
        text.includes("/Users/")
      ) {
        offenders.push(file);
      }
    }
    expect(offenders).toEqual([]);
  });

  // Poodle must be *declared* as a published version, never as a path into a
  // checkout beside this one. A package that ships a path dependency is
  // uninstallable for everyone who is not the author.
  //
  // This has been wrong twice, both times by inspecting the wrong thing.
  // First it realpath'd `../poodle/packages/svelte/components` to compare
  // against, which named a path outside the repository and so passed only
  // where Poodle sat as a sibling -- the first CI run failed here with ENOENT,
  // on the very test meant to forbid depending on sibling source. Then it
  // asserted the install resolved under `node_modules`, which is false the
  // moment anyone runs the sanctioned `effigy deps link bun ../poodle`.
  //
  // Both mistook the developer's node_modules for the published contract. The
  // manifest is the contract: `effigy deps link` leaves it byte-for-byte
  // unchanged by design, so linking Poodle to work against an unreleased
  // version is invisible here, exactly as it should be.
  it("declares Poodle by published version, not by path", () => {
    const pathProtocol = /^(file|link|portal|workspace):/;
    const declarations = [
      [
        "workspace dependency",
        // A devDependency at the root: the packages take Poodle as an optional
        // peer, and the root installs it to build and test against.
        (rootManifest.dependencies?.[POODLE] ??
          rootManifest.devDependencies?.[POODLE]) as string | undefined,
      ],
      ["peer", packageManifest.peerDependencies?.[POODLE] as string | undefined],
    ] as const;

    for (const [kind, spec] of declarations) {
      expect(spec, `${POODLE} is not declared as a ${kind}`).toBeDefined();
      expect(
        pathProtocol.test(spec as string),
        `${POODLE} ${kind} is a path reference: ${spec}`,
      ).toBe(false);
    }
  });
});
