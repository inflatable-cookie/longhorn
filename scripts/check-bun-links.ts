// Reports the machine's bun global-link state, and fails when a workspace
// dependency resolves through a link that leaves the repository.
//
// `bun link` inside a package registers it under
// `~/.bun/install/global/node_modules`; `bun link <name>` inside a consumer
// symlinks `node_modules/<name>` at it. A machine that once linked Poodle can
// keep every local gate green against bytes no consumer resolves -- the
// sibling-checkout hazard arriving through bun's own mechanism, invisible to
// the manifests and `bun.lock` both. `ci:rehearse` runs this so a release
// rehearsal reports the machine's link state instead of assuming it is clean.
//
// Registered-but-unused links poison nothing, so they are reported and the
// check passes. An active resolution that escapes the repository fails: run
// `bun unlink <name>` in the project (and `bun unlink` in the linked package)
// and reinstall.

import { lstat, readdir, realpath } from "node:fs/promises";
import { homedir } from "node:os";
import { join, relative, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");

/** Scopes and top-level names whose resolution must not ride a machine link. */
const SCOPES = ["@inflatable-cookie"];
const TOP_LEVEL = ["svelte"];

/** Where `bun link` registers packages; overridable the way bun allows. */
function globalLinkRoot(): string {
  const install = process.env.BUN_INSTALL ?? join(homedir(), ".bun");
  return join(install, "install", "global", "node_modules");
}

type Link = { name: string; target: string };
const globalLinks: Link[] = [];
const findings: string[] = [];
let resolutionsChecked = 0;

// The global link registry: whatever is registered machine-wide. Bun installs
// ordinary global packages into the same tree, so only symlinks are links.
// Names the workspace never depends on are still listed -- the rehearsal is
// where the operator sees the state, and a stale Poodle link reads as harmless
// right up until something resolves through it.
for (const entry of await readdir(globalLinkRoot(), { withFileTypes: true }).catch(
  () => [],
)) {
  if (entry.name.startsWith(".")) continue;
  const path = join(globalLinkRoot(), entry.name);
  if (entry.isSymbolicLink()) {
    await recordGlobalLink(path, entry.name);
    continue;
  }
  if (!entry.isDirectory() || !SCOPES.includes(entry.name)) continue;
  const scoped = await readdir(path, { withFileTypes: true }).catch(() => []);
  for (const nested of scoped) {
    if (nested.isSymbolicLink()) {
      await recordGlobalLink(join(path, nested.name), `${entry.name}/${nested.name}`);
    }
  }
}

// Workspace resolutions: every top-level node_modules entry under the scopes
// and the published peers, in the root and in each workspace package. Bun's
// own layout (node_modules/.bun/... and workspace cross-links) stays inside
// the repository by design; a link aimed at a sibling checkout does not.
await workspaceScan(join(repoRoot, "node_modules"));
for (const entry of await readdir(join(repoRoot, "packages")).catch(() => [])) {
  await workspaceScan(join(repoRoot, "packages", entry, "node_modules"));
}

if (findings.length > 0) {
  console.error(
    "A dependency in this workspace resolves through a link outside the repository.\n\n" +
      findings.map((line) => `  ${line}`).join("\n") +
      "\n\nA release rehearsal on this machine proves nothing about what a consumer" +
      " resolves. Unlink (`bun unlink <name>`, `bun unlink` in the linked package)" +
      " and run `bun install` again.",
  );
  process.exit(1);
}

console.log(
  JSON.stringify(
    {
      schema: "longhorn.bun-link-state.v1",
      outcome: "pass",
      globalLinkRoot: globalLinkRoot(),
      globalLinks,
      workspaceResolutionsChecked: resolutionsChecked,
    },
    null,
    2,
  ),
);

async function recordGlobalLink(path: string, name: string): Promise<void> {
  const target = await realpath(path).catch(() => "unresolvable");
  globalLinks.push({ name, target });
}

async function workspaceScan(nodeModules: string): Promise<void> {
  const entries = await readdir(nodeModules).catch(() => null);
  if (entries === null) return;
  for (const entry of entries) {
    if (entry === ".bin" || entry.startsWith(".")) continue;
    if (entry.startsWith("@")) {
      if (!SCOPES.includes(entry)) continue;
      const scoped = await readdir(join(nodeModules, entry)).catch(() => null);
      if (scoped === null) continue;
      for (const name of scoped) {
        await inspectResolution(nodeModules, join(entry, name));
      }
      continue;
    }
    if (TOP_LEVEL.includes(entry)) {
      await inspectResolution(nodeModules, entry);
    }
  }
}

async function inspectResolution(nodeModules: string, name: string): Promise<void> {
  const path = join(nodeModules, name);
  const stat = await lstat(path).catch(() => null);
  if (stat === null) return;
  const target = await realpath(path).catch(() => null);
  if (target === null) {
    findings.push(`${display(nodeModules, name)} does not resolve`);
    return;
  }
  const rel = relative(repoRoot, target);
  if (rel === "" || !rel.startsWith("..")) {
    resolutionsChecked += 1;
    return;
  }
  findings.push(
    `${display(nodeModules, name)} resolves outside the repository: ${target}`,
  );
}

/** The node_modules path relative to the repository, for readable findings. */
function display(nodeModules: string, name: string): string {
  return `${relative(repoRoot, nodeModules)}/${name}`;
}
