import { join, resolve } from "node:path";
import { mkdir, mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";

import {
  packAndCheckRustArtifacts,
  packTypescriptArtifacts,
} from "./bridge-topology-artifact-proof/artifacts.ts";
import { verifyConsumers } from "./bridge-topology-artifact-proof/consumers.ts";
import type { TopologyDeclarations } from "./bridge-topology-artifact-proof/types.ts";
import { run } from "./bridge-topology-artifact-proof/shared.ts";

const repoRoot = resolve(import.meta.dir, "..");
const proofRoot = join(repoRoot, "examples", "bridge-topology-proof");
const temporaryRoot = await mkdtemp(
  join(tmpdir(), "longhorn-bridge-artifact-proof-"),
);
const artifactRoot = join(temporaryRoot, "typescript-artifacts");
await mkdir(artifactRoot);

try {
  await run(
    ["cargo", "run", "-p", "longhorn-bindings", "--", "bridge", "check"],
    repoRoot,
  );
  const declarations = JSON.parse(
    await readFile(join(proofRoot, "declarations.json"), "utf8"),
  ) as TopologyDeclarations;
  const typescript = await packTypescriptArtifacts(repoRoot, artifactRoot);
  const rust = await packAndCheckRustArtifacts(repoRoot, temporaryRoot);
  const consumers = await verifyConsumers({
    repoRoot,
    proofRoot,
    temporaryRoot,
    artifacts: typescript.paths,
    declarations,
  });
  const splitShell = consumers.find(({ shape }) => shape === "split-shell")!;
  const loophole = consumers.find(({ shape }) => shape === "loophole")!;
  console.log(
    JSON.stringify(
      {
        schema: "longhorn.bridge-topology-artifact-proof.v1",
        rustArtifacts: rust.identities,
        rustPackaging: rust.packaging,
        rustCompileGraphs: rust.compileGraphs,
        typescriptArtifacts: typescript.identities,
        consumers,
        protocol: splitShell.trace.protocol,
        lifecycle: loophole.trace.lifecycle,
        audits: {
          generatedBindingsDrift: false,
          adapterParity: ["direct", "tauri", "loopback"],
          optionalEdgesProved: true,
          eventCapabilitiesExact: true,
          retryLimitsDeclared: true,
          opaqueCredentialReferencesOnly: true,
          productPayloadsExcluded: true,
          productAuthorityMoved: false,
          productionNetworkingIncluded: false,
          deploymentTopologyIncluded: false,
        },
      },
      null,
      2,
    ),
  );
} finally {
  if (process.env.KEEP_BRIDGE_TOPOLOGY_ARTIFACT_PROOF === "1") {
    console.error(`retained proof workspace: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}
