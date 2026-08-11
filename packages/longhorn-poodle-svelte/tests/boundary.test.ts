import { readFile, readdir, realpath, stat } from "node:fs/promises";
import { join, resolve } from "node:path";

import { describe, expect, it } from "vitest";

const packageRoot = resolve(process.cwd(), "packages/longhorn-poodle-svelte");
const repositoryRoot = resolve(packageRoot, "../..");

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

  // Poodle must come from the registry, never from a checkout beside this one.
  //
  // This used to prove it by realpath-ing `../poodle/packages/svelte/components`
  // and asserting the install differed from it. That named a path outside the
  // repository, so it passed only on a machine with Poodle checked out as a
  // sibling: the first CI run failed here with ENOENT, on the very test meant
  // to forbid depending on sibling source.
  //
  // Containment is the property that was actually wanted, and it is stronger.
  // Any escaping pin -- the old `file:../poodle/.artifacts/...`, a `link:`, an
  // npm link -- resolves outside `node_modules`, and needs no donor to detect.
  it("resolves an installed Poodle, not a link to source outside the repository", async () => {
    const installedRoot = await realpath(
      resolve(repositoryRoot, "node_modules/@inflatable-cookie/poodle-svelte"),
    );
    const modulesRoot = await realpath(resolve(repositoryRoot, "node_modules"));
    const metadata = JSON.parse(
      await readFile(resolve(installedRoot, "package.json"), "utf8"),
    );

    expect(installedRoot.startsWith(`${modulesRoot}/`)).toBe(true);
    expect(metadata.name).toBe("@inflatable-cookie/poodle-svelte");
    expect(metadata.version).toBe("0.1.0");
  });
});
