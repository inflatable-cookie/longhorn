import { parseJsonRecord, requireList, requireRecord, requireString } from "./proof-json.ts";
import { requireValue } from "./proof-support.ts";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { parseReleaseGates } from "./check-release-gates-alignment.ts";
import { assert, repoRoot } from "./private-candidate-docs/support";

export type CandidateFacts = {
  version: string;
  rustCrates: number;
  typescriptPackages: number;
  consumerGraphs: number;
};

type FactField = keyof CandidateFacts;

const SMALL_COUNT_WORDS = [
  "zero",
  "one",
  "two",
  "three",
  "four",
  "five",
  "six",
  "seven",
  "eight",
  "nine",
  "ten",
  "eleven",
  "twelve",
] as const;

const GATE_SCRIPT = "bun scripts/verify-private-candidate-docs.ts";

export function factsFromReceipt(receipt: {
  version?: unknown;
  artifacts?: {
    longhornTypescript?: { packages?: unknown };
    longhornRust?: { packages?: unknown };
  };
  graphs?: unknown;
}): CandidateFacts {
  assert(
    typeof receipt.version === "string" && receipt.version.length > 0,
    "receipt no longer records version",
  );
  const rustPackages = receipt.artifacts?.longhornRust?.packages;
  assert(
    Array.isArray(rustPackages) && rustPackages.length > 0,
    "receipt no longer records Rust packages",
  );
  const typescriptPackages = receipt.artifacts?.longhornTypescript?.packages;
  assert(
    Array.isArray(typescriptPackages) && typescriptPackages.length > 0,
    "receipt no longer records TypeScript packages",
  );
  assert(
    Array.isArray(receipt.graphs) && receipt.graphs.length > 0,
    "receipt no longer records consumer graphs",
  );
  return {
    version: receipt.version,
    rustCrates: rustPackages.length,
    typescriptPackages: typescriptPackages.length,
    consumerGraphs: receipt.graphs.length,
  };
}

function foldProse(text: string): string {
  return text.replaceAll("`", "").replaceAll(/\s+/g, " ").trim();
}

export function assertCandidateFactsInProse(
  label: string,
  text: string,
  facts: CandidateFacts,
  fields: ReadonlyArray<FactField>,
): void {
  const folded = foldProse(text);
  for (const field of fields) {
    if (field === "version") {
      const version = facts.version.replaceAll(".", String.raw`\.`);
      assert(
        new RegExp(`${version}.{0,80}candidate|candidate.{0,80}${version}`, "i").test(folded),
        `${label} lacks candidate version ${facts.version}`,
      );
      continue;
    }
    if (field === "rustCrates") {
      assert(
        hasCountNear(folded, facts.rustCrates, "Rust"),
        `${label} lacks ${facts.rustCrates} Rust crates`,
      );
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

function artifactRecord(value: unknown, name: string) {
  const artifact = requireRecord(value, name);
  return {
    setSha256: requireString(artifact.setSha256, `${name} hash`),
    packages: requireList(artifact.packages, `${name} packages`),
  };
}

function sourceCommit(value: unknown): string {
  return requireString(requireRecord(value, "source").commit, "source commit");
}

export async function verifyPrivateCandidateDocs(root = repoRoot): Promise<{
  schema: string;
  outcome: string;
  candidateVersion: string;
  consumerGraphs: number;
  protocolFixtures: number;
  publication: boolean;
}> {
  const data = parseJsonRecord(
    await readFile(join(root, "fixtures/release/private-0-1-candidate-v1.json"), "utf8"),
  );
  const sources = requireRecord(data.sources, "receipt sources");
  const artifacts = requireRecord(data.artifacts, "receipt artifacts");
  const protocols = requireRecord(data.compatibility, "receipt compatibility");
  const receipt = {
    version: requireString(data.version, "receipt version"),
    sources: {
      longhorn: sourceCommit(sources.longhorn),
      poodle: sourceCommit(sources.poodle),
      consumers: Object.values(requireRecord(sources.consumers, "consumer sources")).map(
        sourceCommit,
      ),
    },
    artifacts: {
      longhornTypescript: artifactRecord(artifacts.longhornTypescript, "Longhorn TypeScript"),
      poodle: artifactRecord(artifacts.poodle, "Poodle"),
      longhornRust: artifactRecord(artifacts.longhornRust, "Longhorn Rust"),
    },
    compatibility: {
      protocols: requireList(protocols.protocols, "protocols"),
      protocolNegotiation: protocols.protocolNegotiation,
    },
    graphs: requireList(data.graphs, "consumer graphs").map((value) => {
      const graph = requireRecord(value, "consumer graph");
      return {
        name: requireString(graph.name, "consumer name"),
        typescriptPackages: requireList(graph.typescriptPackages, "TypeScript graph"),
        rustDirectPackages: requireList(graph.rustDirectPackages, "direct Rust graph"),
        rustResolvedPackages: requireList(graph.rustResolvedPackages, "resolved Rust graph"),
      };
    }),
    audits: requireRecord(data.audits, "receipt audits"),
  };
  const facts = factsFromReceipt(receipt);
  const reference = await readFile(join(root, "docs/reference/private-0-1-candidate.md"), "utf8");
  const compatibility = await readFile(
    join(root, "docs/guides/compatibility-and-upgrades.md"),
    "utf8",
  );
  const changelog = await readFile(join(root, "CHANGELOG.md"), "utf8");
  const releaseToml = await readFile(join(root, "config/release.toml"), "utf8");

  requireAll(reference, [
    receipt.version,
    receipt.sources.longhorn,
    receipt.sources.poodle,
    ...receipt.sources.consumers,
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
  assertCandidateFactsInProse("compatibility-and-upgrades.md", compatibility, facts, [
    "version",
    "consumerGraphs",
  ]);
  requireAll(compatibility, ["read-only gates only"]);
  assert(
    releaseToml.includes('version-path = "workspace.package.version"'),
    "release.toml lacks version-path",
  );
  const gates = parseReleaseGates(releaseToml);
  const privateCandidate = gates.find((gate) => gate.name === "private-candidate");
  assert(
    privateCandidate?.command === GATE_SCRIPT,
    `release.toml private-candidate must be ${GATE_SCRIPT}`,
  );
  const workspace = gates.find((gate) => gate.name === "workspace");
  assert(workspace?.command === "effigy qa", 'release.toml workspace must be "effigy qa"');
  assert(receipt.compatibility.protocols.length === 12, "protocol fixture count drift");
  assert(receipt.compatibility.protocolNegotiation === "exact-v1", "protocol negotiation drift");
  for (const [name, value] of Object.entries(receipt.audits)) {
    if (
      [
        "siblingWorkspaceResolution",
        "consumerRepositoryWrites",
        "packageManagerPublication",
        "gitTags",
        "hostedReleases",
      ].includes(name)
    ) {
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

function requireAll(content: string, values: Array<string>): void {
  for (const value of values) {
    assert(content.includes(value), `human surface lacks ${value}`);
  }
}

function displayName(name: string): string {
  if (name === "optional-server") {
    return "optional server";
  }
  if (["nucleus", "loophole", "soundcheck", "split-shell", "jetstream"].includes(name)) {
    return `${requireValue(name[0], "name[0]").toUpperCase()}${name.slice(1)}`;
  }
  return name;
}
