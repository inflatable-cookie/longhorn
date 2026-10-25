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
import { rustCommand, taskScratch } from "./rust-command.ts";

const repoRoot = resolve(import.meta.dir, "..");
const proofRoot = join(repoRoot, "examples", "bridge-topology-proof");
const temporaryRoot = await mkdtemp(
  join(tmpdir(), "longhorn-bridge-artifact-proof-"),
);
const artifactRoot = join(temporaryRoot, "typescript-artifacts");
await mkdir(artifactRoot);
const buildScratch = await taskScratch("longhorn-bridge-topology-rust");

try {
  // Checkout-level and non-building: the bindings check runs through the switch
  // point and pools. Only the staged-workspace builds keep private outputs.
  await run(
    rustCommand(["run", "-p", "longhorn-bindings", "--", "bridge", "check"]),
    repoRoot,
  );
  const declarations = JSON.parse(
    await readFile(join(proofRoot, "declarations.json"), "utf8"),
  ) as TopologyDeclarations;
  const typescript = await packTypescriptArtifacts(repoRoot, artifactRoot);
  const rust = await packAndCheckRustArtifacts(
    repoRoot,
    temporaryRoot,
    buildScratch.root,
  );
  const consumers = await verifyConsumers({
    repoRoot,
    proofRoot,
    temporaryRoot,
    artifacts: typescript.paths,
    declarations,
  });
  const minimalShell = consumers.find(({ shape }) => shape === "minimal-shell")!;
  const reconnectingLifecycle = consumers.find(({ shape }) => shape === "reconnecting-lifecycle")!;
  console.log(
    JSON.stringify(
      {
        schema: "longhorn.bridge-topology-artifact-proof.v1",
        rustArtifacts: rust.identities,
        rustPackaging: rust.packaging,
        rustCompileGraphs: rust.compileGraphs,
        typescriptArtifacts: typescript.identities,
        consumers,
        protocol: minimalShell.trace.protocol,
        lifecycle: reconnectingLifecycle.trace.lifecycle,
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
  await buildScratch.dispose();
}
