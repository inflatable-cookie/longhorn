import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { assert, repoRoot } from "./private-candidate-card127/support";

const receipt = JSON.parse(await readFile(join(repoRoot, "fixtures/release/card127/private-0-1-candidate-v1.json"), "utf8")) as {
  version: string;
  sources: {
    longhorn: { commit: string };
    poodle: { commit: string };
    consumers: Record<string, { commit: string }>;
  };
  artifacts: {
    longhornTypescript: { setSha256: string; packages: unknown[] };
    poodle: { setSha256: string; packages: unknown[] };
    longhornRust: { setSha256: string; packages: unknown[] };
  };
  compatibility: { protocols: unknown[]; protocolNegotiation: string };
  graphs: Array<{ name: string; typescriptPackages: unknown[]; rustDirectPackages: unknown[]; rustResolvedPackages: unknown[] }>;
  audits: Record<string, boolean>;
};
const reference = await readFile(join(repoRoot, "docs/reference/private-0-1-candidate.md"), "utf8");
const compatibility = await readFile(join(repoRoot, "docs/guides/compatibility-and-upgrades.md"), "utf8");
const changelog = await readFile(join(repoRoot, "CHANGELOG.md"), "utf8");
const effigy = await readFile(join(repoRoot, "effigy.toml"), "utf8");

requireAll(reference, [
  receipt.version,
  receipt.sources.longhorn.commit,
  receipt.sources.poodle.commit,
  ...Object.values(receipt.sources.consumers).map(({ commit }) => commit),
  receipt.artifacts.longhornTypescript.setSha256,
  receipt.artifacts.poodle.setSha256,
  receipt.artifacts.longhornRust.setSha256,
  `${receipt.artifacts.longhornTypescript.packages.length}`,
  `${receipt.artifacts.poodle.packages.length}`,
  `${receipt.artifacts.longhornRust.packages.length}`,
  "registry-normalized `.crate` files",
  "Mutating prepare/execute commands remain outside",
]);
for (const graph of receipt.graphs) {
  const row = `| ${displayName(graph.name)} | ${graph.typescriptPackages.length} | ${graph.rustDirectPackages.length} | ${graph.rustResolvedPackages.length} |`;
  assert(reference.includes(row), `candidate reference lacks graph row ${row}`);
}
requireAll(compatibility, ["private `0.1.0` candidate", "seven consumer", "read-only gates only"]);
requireAll(changelog, ["deterministic private `0.1.0` candidate", "17 TypeScript", "36 Rust", "seven consumer"]);
requireAll(effigy, [
  'version-path = "workspace.package.version"',
  'private-candidate = "bun scripts/verify-private-candidate-docs-card127.ts"',
  'workspace = "effigy qa"',
]);
assert(receipt.compatibility.protocols.length === 12, "protocol fixture count drift");
assert(receipt.compatibility.protocolNegotiation === "exact-v1", "protocol negotiation drift");
for (const [name, value] of Object.entries(receipt.audits)) {
  if (["siblingWorkspaceResolution", "consumerRepositoryWrites", "packageManagerPublication", "gitTags", "hostedReleases"].includes(name)) {
    assert(value === false, `${name} must remain false`);
  }
}

console.log(JSON.stringify({
  schema: "longhorn.card127-candidate-doc-proof.v1",
  outcome: "pass",
  candidateVersion: receipt.version,
  consumerGraphs: receipt.graphs.length,
  protocolFixtures: receipt.compatibility.protocols.length,
  publication: false,
}, null, 2));

function requireAll(content: string, values: string[]): void {
  for (const value of values) assert(content.includes(value), `human surface lacks ${value}`);
}

function displayName(name: string): string {
  if (name === "optional-server") return "optional server";
  if (["nucleus", "loophole", "soundcheck", "split-shell", "jetstream"].includes(name)) {
    return `${name[0]!.toUpperCase()}${name.slice(1)}`;
  }
  return name;
}
