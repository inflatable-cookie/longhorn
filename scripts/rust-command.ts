// Scripts that run Cargo themselves build their argv here instead of naming a
// runner. `scripts/rust-command.sh` is Longhorn's one switch point (`oxide`
// locally, plain `cargo` when `GITHUB_ACTIONS=true`), so the CI-aware rule and
// the loud missing-Oxide failure have exactly one implementation. This module
// adds no rule of its own; it only names that script.

import { join, resolve } from "node:path";

const repositoryRoot = resolve(import.meta.dir, "..");
const switchPoint = join(repositoryRoot, "scripts", "rust-command.sh");

/** Full argv for the given Cargo arguments, run through the switch point. */
export function rustCommand(args: readonly string[]): string[] {
  return ["bash", switchPoint, ...args];
}
