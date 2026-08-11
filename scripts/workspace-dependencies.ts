// The artifact proofs build a throwaway Cargo workspace, copy some crates into
// it and check that a consumer compiles. That synthetic workspace needs a
// `[workspace.dependencies]` table, and each proof used to carry its own hand
// written copy of the real one.
//
// Three of them drifted. By 2026-08-11 the copies pinned zip 5.1.1 against the
// workspace's 8.6.0, rusqlite 0.31.0 against 0.40.1, ts-rs 11.0.0 against
// 11.1.0 and sha2 0.10.x against 0.11.0 -- nine stale pins in total.
//
// Locally that is invisible: `cargo check --offline` finds the old versions in
// ~/.cargo from some earlier build, so the proof passes. On a runner whose
// cache holds only what the real lockfile asked for, it cannot resolve them:
//
//     error: no matching package named `arbitrary` found
//     required by package `zip v5.1.1`
//
// `arbitrary` is an optional dependency of zip 5, which nothing in this
// repository has fetched since the 8.6.0 bump.
//
// So the table is read from the real manifest instead of copied. A version
// bump in Cargo.toml now reaches the proofs with no second edit, and there is
// nothing left to drift.

import { readFileSync } from "node:fs";
import { join } from "node:path";

const repositoryRoot = join(import.meta.dir, "..");

function workspaceDependencyTable(): Map<string, string> {
  const manifest = readFileSync(join(repositoryRoot, "Cargo.toml"), "utf8");
  const start = manifest.indexOf("[workspace.dependencies]");
  if (start === -1) {
    throw new Error("Cargo.toml has no [workspace.dependencies] table");
  }
  const body = manifest.slice(start + "[workspace.dependencies]".length);
  const end = body.search(/^\[/m);
  const table = end === -1 ? body : body.slice(0, end);

  const entries = new Map<string, string>();
  for (const line of table.split("\n")) {
    const match = /^([A-Za-z0-9_-]+) = (.+)$/.exec(line.trim());
    if (match) entries.set(match[1], line.trim());
  }
  return entries;
}

/**
 * Returns the named entries from the workspace dependency table, verbatim and
 * in the order given, ready to paste into a generated manifest.
 *
 * Throws on a name the table does not define. A proof asking for a dependency
 * that has been renamed or dropped should fail loudly here rather than emit a
 * manifest missing it and fail later as a confusing resolution error.
 */
export function workspaceDependencies(names: readonly string[]): string {
  const table = workspaceDependencyTable();
  return names
    .map((name) => {
      const entry = table.get(name);
      if (!entry) {
        throw new Error(
          `[workspace.dependencies] does not define ${name}; ` +
            `known: ${[...table.keys()].join(", ")}`,
        );
      }
      return entry;
    })
    .join("\n");
}
