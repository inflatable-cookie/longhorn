// Shared absence assertions for the artifact proofs.
//
// Card 164 collapsed eighteen TypeScript packages into three, which changed
// what "this consumer does not take layout" can mean. It used to be
// install-absence: no `node_modules/@inflatable-cookie/longhorn-layout`. Layout
// now ships inside `@inflatable-cookie/longhorn` whether a consumer composes it
// or not, so that directory can never exist and the old check would pass
// vacuously — which is worse than failing, because it reads as evidence.
//
// Split the claim in two:
//
//   - a *package* the consumer must not resolve  -> assertPackageAbsent
//   - a *subpath* the consumer must not import   -> assertImportsAbsent
//
// Anything containing a `/` after the package name is a subpath and belongs to
// the second. Passing one to the other is the mistake this module exists to
// prevent, so it refuses rather than silently degrading.

import { lstat, readdir, readFile } from "node:fs/promises";
import { join } from "node:path";

const SCOPE = "@inflatable-cookie/";
const PACKAGES = new Set([
  `${SCOPE}longhorn`,
  `${SCOPE}longhorn-poodle-svelte`,
  `${SCOPE}longhorn-tauri`,
]);

export function isSubpath(specifier: string): boolean {
  return !PACKAGES.has(specifier);
}

export async function assertPackageAbsent(
  stage: string,
  name: string,
): Promise<void> {
  if (isSubpath(name)) {
    throw new Error(
      `${name} is a subpath, not a package: assert import-absence instead`,
    );
  }
  try {
    await lstat(join(stage, "node_modules", ...name.split("/")));
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === "ENOENT") return;
    throw error;
  }
  throw new Error(`${name} unexpectedly entered the install graph`);
}

async function consumerSources(stage: string): Promise<string> {
  const roots = ["src", "."];
  for (const root of roots) {
    try {
      const entries = (await readdir(join(stage, root), {
        recursive: true,
      })) as string[];
      const files = entries.filter(
        (path) =>
          /\.(ts|svelte)$/.test(path) && !path.startsWith("node_modules"),
      );
      if (files.length === 0) continue;
      const sources = await Promise.all(
        files.map((path) => readFile(join(stage, root, path), "utf8")),
      );
      return sources.join("\n");
    } catch (error) {
      if ((error as NodeJS.ErrnoException).code !== "ENOENT") throw error;
    }
  }
  throw new Error(`no consumer sources found under ${stage}`);
}

export async function assertImportsAbsent(
  stage: string,
  specifiers: readonly string[],
): Promise<void> {
  if (specifiers.length === 0) return;
  const sources = await consumerSources(stage);
  for (const specifier of specifiers) {
    if (sources.includes(specifier)) {
      throw new Error(`${specifier} unexpectedly imported by the consumer`);
    }
  }
}

/** Split a mixed forbidden list into the two claims it now makes. */
export function splitForbidden(
  forbidden: readonly string[],
): { readonly packages: string[]; readonly imports: string[] } {
  return {
    packages: forbidden.filter((name) => !isSubpath(name)),
    imports: forbidden.filter((name) => isSubpath(name)),
  };
}

/** Exactly these Longhorn packages resolve in the graph, and no others. */
export async function assertLonghornGraphExact(
  stage: string,
  expected: readonly string[],
): Promise<void> {
  const scope = join(stage, "node_modules", "@inflatable-cookie");
  const installed = (await readdir(scope))
    .filter((entry) => entry === "longhorn" || entry.startsWith("longhorn-"))
    .map((entry) => `${SCOPE}${entry}`)
    .sort();
  const want = [...expected].sort();
  if (JSON.stringify(installed) !== JSON.stringify(want)) {
    throw new Error(
      `install graph carries ${installed.join(", ")}; expected ${want.join(", ")}`,
    );
  }
}
