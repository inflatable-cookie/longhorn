import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { parseReleaseGates } from "./check-release-gates-alignment.ts";
import { assert, repoRoot } from "./private-candidate-card127/support";

export type CandidateFacts = {
  version: string;
  rustCrates: number;
  typescriptPackages: number;
  consumerGraphs: number;
};

type CandidateReceipt = {
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
  graphs: Array<{
    name: string;
    typescriptPackages: unknown[];
    rustDirectPackages: unknown[];
    rustResolvedPackages: unknown[];
  }>;
  audits: Record<string, boolean>;
};

type FactField = keyof CandidateFacts;

const SMALL_COUNT_WORDS: Readonly<Record<number, string>> = {
  0: "zero",
  1: "one",
  2: "two",
  3: "three",
  4: "four",
  5: "five",
  6: "six",
  7: "seven",
  8: "eight",
  9: "nine",
  10: "ten",
  11: "eleven",
  12: "twelve",
};

const GATE_SCRIPT = "bun scripts/verify-private-candidate-docs-card127.ts";

export function factsFromReceipt(receipt: {
  version?: unknown;
  artifacts?: {
    longhornTypescript?: { packages?: unknown };
    longhornRust?: { packages?: unknown };
  };
  graphs?: unknown;
}): CandidateFacts {
  assert(typeof receipt.version === "string" && receipt.version.length > 0, "receipt no longer records version");
  const rustPackages = receipt.artifacts?.longhornRust?.packages;
  assert(Array.isArray(rustPackages) && rustPackages.length > 0, "receipt no longer records Rust packages");
  const typescriptPackages = receipt.artifacts?.longhornTypescript?.packages;
  assert(
    Array.isArray(typescriptPackages) && typescriptPackages.length > 0,
    "receipt no longer records TypeScript packages",
  );
  assert(Array.isArray(receipt.graphs) && receipt.graphs.length > 0, "receipt no longer records consumer graphs");
  return {
    version: receipt.version,
    rustCrates: rustPackages.length,
    typescriptPackages: typescriptPackages.length,
    consumerGraphs: receipt.graphs.length,
  };
}

function foldProse(text: string): string {
  return text.replaceAll("`", "").replace(/\s+/g, " ").trim();
}

export function assertCandidateFactsInProse(
  label: string,
  text: string,
  facts: CandidateFacts,
  fields: readonly FactField[],
): void {
  const folded = foldProse(text);
  for (const field of fields) {
    if (field === "version") {
      const version = facts.version.replaceAll(".", "\\.");
      assert(
        new RegExp(`${version}.{0,80}candidate|candidate.{0,80}${version}`, "i").test(folded),
        `${label} lacks candidate version ${facts.version}`,
      );
      continue;
    }
    if (field === "rustCrates") {
      assert(hasCountNear(folded, facts.rustCrates, "Rust"), `${label} lacks ${facts.rustCrates} Rust crates`);
      continue;
    }
    if (field === "typescriptPackages") {
      assert(
        hasCountNear(folded, facts.typescriptPackages, "TypeScript"),
        `${label} lacks ${facts.typescriptPackages} TypeScript packages`,
      );
      continue;
    }
    assert(
      hasCountNear(folded, facts.consumerGraphs, "consumers?"),
      `${label} lacks ${facts.consumerGraphs} consumer graphs`,
    );
  }
}

export async function verifyPrivateCandidateDocs(root = repoRoot): Promise<{
  schema: string;
  outcome: string;
  candidateVersion: string;
  consumerGraphs: number;
  protocolFixtures: number;
  publication: boolean;
}> {
  const receipt = JSON.parse(
    await readFile(join(root, "fixtures/release/card127/private-0-1-candidate-v1.json"), "utf8"),
  ) as CandidateReceipt;
  const facts = factsFromReceipt(receipt);
  const reference = await readFile(join(root, "docs/reference/private-0-1-candidate.md"), "utf8");
  const compatibility = await readFile(join(root, "docs/guides/compatibility-and-upgrades.md"), "utf8");
  const changelog = await readFile(join(root, "CHANGELOG.md"), "utf8");
  const releaseToml = await readFile(join(root, "config/release.toml"), "utf8");

  requireAll(reference, [
    receipt.version,
    receipt.sources.longhorn.commit,
    receipt.sources.poodle.commit,
    ...Object.values(receipt.sources.consumers).map(({ commit }) => commit),
    receipt.artifacts.longhornTypescript.setSha256,
    receipt.artifacts.poodle.setSha256,
    receipt.artifacts.longhornRust.setSha256,
    `${facts.typescriptPackages}`,
    `${receipt.artifacts.poodle.packages.length}`,
    `${facts.rustCrates}`,
    "registry-normalized `.crate` files",
    "Mutating prepare/execute commands remain outside",
  ]);
  for (const graph of receipt.graphs) {
    const row = `| ${displayName(graph.name)} | ${graph.typescriptPackages.length} | ${graph.rustDirectPackages.length} | ${graph.rustResolvedPackages.length} |`;
    assert(reference.includes(row), `candidate reference lacks graph row ${row}`);
  }
  assertCandidateFactsInProse("CHANGELOG.md", changelog, facts, [
    "version",
    "typescriptPackages",
    "rustCrates",
    "consumerGraphs",
  ]);
  assertCandidateFactsInProse("compatibility-and-upgrades.md", compatibility, facts, ["version", "consumerGraphs"]);
  requireAll(compatibility, ["read-only gates only"]);
  assert(releaseToml.includes('version-path = "workspace.package.version"'), "release.toml lacks version-path");
  const gates = parseReleaseGates(releaseToml);
  const privateCandidate = gates.find((gate) => gate.name === "private-candidate");
  assert(privateCandidate?.command === GATE_SCRIPT, `release.toml private-candidate must be ${GATE_SCRIPT}`);
  const workspace = gates.find((gate) => gate.name === "workspace");
  assert(workspace?.command === "effigy qa", 'release.toml workspace must be "effigy qa"');
  assert(receipt.compatibility.protocols.length === 12, "protocol fixture count drift");
  assert(receipt.compatibility.protocolNegotiation === "exact-v1", "protocol negotiation drift");
  for (const [name, value] of Object.entries(receipt.audits)) {
    if (["siblingWorkspaceResolution", "consumerRepositoryWrites", "packageManagerPublication", "gitTags", "hostedReleases"].includes(name)) {
      assert(value === false, `${name} must remain false`);
    }
  }

  return {
    schema: "longhorn.card127-candidate-doc-proof.v1",
    outcome: "pass",
    candidateVersion: facts.version,
    consumerGraphs: facts.consumerGraphs,
    protocolFixtures: receipt.compatibility.protocols.length,
    publication: false,
  };
}

if (import.meta.main) {
  console.log(JSON.stringify(await verifyPrivateCandidateDocs(), null, 2));
}

function hasCountNear(folded: string, count: number, noun: string): boolean {
  const word = SMALL_COUNT_WORDS[count];
  const tokens = word ? `${count}|${word}` : String(count);
  return new RegExp(`\\b(?:${tokens})\\s+${noun}\\b`, "i").test(folded);
}

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
