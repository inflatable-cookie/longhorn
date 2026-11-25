import { readPermissions } from "./proof-json.ts";
import { requireValue } from "./proof-support.ts";
import { readFile, readdir } from "node:fs/promises";
import { join, resolve } from "node:path";

import { runBridgeTopologyProof } from "../examples/bridge-topology-proof/proof.ts";
import { declaration } from "../examples/bridge-topology-proof/common.ts";
import type { ShapeName } from "../examples/bridge-topology-proof/common.ts";

const repoRoot = resolve(import.meta.dir, "..");
const proofRoot = join(repoRoot, "examples/bridge-topology-proof");
const shapes: ReadonlyArray<ShapeName> = [
  "minimal-shell",
  "ordered-streams",
  "jobs-and-service-failure",
  "capability-authority",
  "reconnecting-lifecycle",
];

const importGraph = Object.fromEntries(
  await Promise.all(
    shapes.map(async (shape) => {
      const imports = await packageImports(join(proofRoot, `${shape}.ts`));
      assertEqualSet(`${shape} Longhorn imports`, imports, declaration(shape).imports);
      return [shape, imports] as const;
    }),
  ),
);

for (const shape of shapes) {
  const fixture = declaration(shape);
  for (const domain of fixture.domains) {
    if (!domain.domainId.startsWith("fixture.")) {
      throw new Error(`${shape} contains non-fixture domain ${domain.domainId}`);
    }
  }
  const imports = requireValue(importGraph[shape], "importGraph[shape]");
  if (fixture.tauriPermissions.some((permission) => permission.startsWith("core:event:"))) {
    throw new Error(`${shape} grants Tauri event permissions for renderer-local subscriptions`);
  }
  const hasService = imports.includes("@inflatable-cookie/longhorn/bridge/supervision");
  if (hasService !== (fixture.serviceOwnership !== null)) {
    throw new Error(`${shape} service import and ownership diverge`);
  }
}

const queryCapability = readPermissions(
  await readFile(
    join(repoRoot, "crates/longhorn-tauri-bridge/examples/capabilities/query-only.json"),
    "utf8",
  ),
);
assertEqualSet(
  "Minimal shell query-only Tauri admission",
  queryCapability.permissions,
  declaration("minimal-shell").tauriPermissions,
);
const subscriptionCapability = readPermissions(
  await readFile(
    join(repoRoot, "crates/longhorn-tauri-bridge/examples/capabilities/subscription.json"),
    "utf8",
  ),
);
assertEqualSet(
  "Bridge subscription invoke permissions",
  subscriptionCapability.permissions,
  declaration("jobs-and-service-failure").tauriPermissions,
);

const rootBarrel = await readFile(join(repoRoot, "packages/longhorn/src/bridge/index.ts"), "utf8");
for (const optional of ["./stream", "./supervision", "./tauri-events"]) {
  if (rootBarrel.includes(optional)) {
    throw new Error(`bridge root barrel imports optional seam ${optional}`);
  }
}

const sharedSources = await sourceText([
  join(repoRoot, "packages/longhorn/src/bridge"),
  join(repoRoot, "crates/longhorn-bridge/src"),
  join(repoRoot, "crates/longhorn-tauri-bridge/src"),
]);
for (const donor of shapes) {
  if (new RegExp(`\\b${donor}\\b`, "i").test(sharedSources)) {
    throw new Error(`shared bridge source contains donor name ${donor}`);
  }
}

const proofSources = (
  await Promise.all(
    [
      "README.md",
      "declarations.json",
      "common.ts",
      "minimal-shell.ts",
      "ordered-streams.ts",
      "jobs-and-service-failure.ts",
      "capability-authority.ts",
      "reconnecting-lifecycle.ts",
      "proof.ts",
      "proof.test.ts",
    ].map((path) => readFile(join(proofRoot, path), "utf8")),
  )
).join("\n");
for (const productionEdge of [
  "@tauri-apps/",
  "WebSocket",
  "TcpStream",
  "fetch(",
  "http://",
  "https://",
]) {
  if (proofSources.includes(productionEdge)) {
    throw new Error(`topology proof contains production edge ${productionEdge}`);
  }
}
if (/\b(token|password|secret)\b/i.test(proofSources)) {
  throw new Error("topology proof contains credential vocabulary");
}

const manifest = await readFile(join(repoRoot, "crates/longhorn-bridge/Cargo.toml"), "utf8");
if (!manifest.includes("supervision = []") || /^\s*default\s*=/m.test(manifest)) {
  throw new Error("Rust supervision is not an explicit opt-in feature");
}

const proof = await runBridgeTopologyProof();
console.log(
  JSON.stringify(
    {
      ...proof,
      audits: {
        ...proof.audits,
        exactImportGraphs: true,
        capabilityGraphMatchesImports: true,
        queryOnlyCapabilityExact: true,
        rootOptionalImportsAbsent: true,
        donorNamesInSharedSources: false,
        credentialVocabularyInFixtures: false,
        rustSupervisionOptIn: true,
      },
    },
    null,
    2,
  ),
);

async function packageImports(entry: string): Promise<ReadonlyArray<string>> {
  const visited = new Set<string>();
  const packages = new Set<string>();

  async function visit(path: string): Promise<void> {
    if (visited.has(path)) {
      return;
    }
    visited.add(path);
    const source = await readFile(path, "utf8");
    const pattern = /(?:import|export)\s+(?:[^"']*?\s+from\s+)?["']([^"']+)["']/g;
    for (const match of source.matchAll(pattern)) {
      const specifier = requireValue(match[1], "match[1]");
      if (/^@inflatable-cookie\/longhorn(\/|-)/.test(specifier)) {
        packages.add(specifier);
      } else if (specifier.startsWith(".")) {
        await visit(resolve(path, "..", specifier));
      }
    }
  }

  await visit(entry);
  return [...packages].sort();
}

async function sourceText(roots: ReadonlyArray<string>): Promise<string> {
  const paths: Array<string> = [];
  for (const root of roots) {
    paths.push(...(await files(root)));
  }
  return (
    await Promise.all(
      paths.flatMap((path) =>
        /\.(rs|ts|json|toml|md)$/.test(path) ? [readFile(path, "utf8")] : [],
      ),
    )
  ).join("\n");
}

async function files(root: string): Promise<Array<string>> {
  const entries = await readdir(root, { withFileTypes: true });
  const output: Array<string> = [];
  for (const entry of entries) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) {
      output.push(...(await files(path)));
    } else if (entry.isFile()) {
      output.push(path);
    }
  }
  return output;
}

function assertEqualSet(
  label: string,
  actual: ReadonlyArray<string>,
  expected: ReadonlyArray<string>,
): void {
  const left = [...actual].sort();
  const right = [...expected].sort();
  if (JSON.stringify(left) !== JSON.stringify(right)) {
    throw new Error(`${label} mismatch: ${JSON.stringify(left)} != ${JSON.stringify(right)}`);
  }
}
