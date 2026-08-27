// Fail fast when TypeScript or Svelte checks run before Bun dependencies exist.
// Fresh worktrees reach `check:ts` with no `node_modules`; `bun x tsc` then
// fetches the compiler and leaves every workspace import unresolved.

import { access } from "node:fs/promises";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const markers = [
  "node_modules/typescript",
  "node_modules/svelte",
  "node_modules/@inflatable-cookie/poodle-core",
];

for (const marker of markers) {
  try {
    await access(join(repoRoot, marker));
  } catch {
    console.error(
      [
        "Bun dependencies are not installed.",
        "",
        "Run: effigy bootstrap:deps",
        "",
        "Fresh Git worktrees need the locked install before TypeScript checks.",
      ].join("\n"),
    );
    process.exit(1);
  }
}

console.log(
  JSON.stringify(
    {
      schema: "longhorn.bun-deps.v1",
      outcome: "pass",
      markers,
    },
    null,
    2,
  ),
);
