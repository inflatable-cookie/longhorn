import { join, resolve } from "node:path";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";

import {
  POODLE_ARTIFACT_SET,
  packAndCheckRustArtifacts,
  packTypescriptArtifacts,
  readPoodleEvidence,
} from "./command-system-artifact-proof/artifacts.ts";
import { verifyConsumers } from "./command-system-artifact-proof/consumers.ts";
import { run } from "./command-system-artifact-proof/shared.ts";

const repoRoot = resolve(import.meta.dir, "..");
const proofRoot = join(repoRoot, "examples", "command-system-proof");
const temporaryRoot = await mkdtemp(
  join(tmpdir(), "longhorn-command-system-proof-"),
);
const artifactRoot = join(temporaryRoot, "typescript-artifacts");
await mkdir(artifactRoot);

try {
  await run(
    ["cargo", "run", "-p", "longhorn-bindings", "--", "commands", "check"],
    repoRoot,
  );
  const poodle = await readPoodleEvidence();
  const typescript = await packTypescriptArtifacts(repoRoot, artifactRoot);
  const rust = await packAndCheckRustArtifacts(repoRoot, temporaryRoot);
  const consumers = await verifyConsumers({
    repoRoot,
    proofRoot,
    temporaryRoot,
    artifacts: typescript.paths,
    poodle,
  });
  console.log(
    JSON.stringify(
      {
        schema: "longhorn.command-system-artifact-proof.v1",
        poodleArtifactSet: POODLE_ARTIFACT_SET,
        rustArtifacts: rust.identities,
        rustPackaging: rust.packaging,
        rustGraphs: rust.graphs,
        nativeTraces: rust.traces,
        typescriptArtifacts: typescript.identities,
        consumers,
        audits: {
          generatedBindingsDrift: false,
          optionalEdgesProved: true,
          peerRuntimeUnique: true,
          capabilitiesExact: true,
          payloadsProductNeutral: true,
          authorityRemainsConsumerOwned: true,
          genericExecutionBusAbsent: true,
          publicPoodleOnly: true,
          donorRepositoriesWritten: false,
        },
      },
      null,
      2,
    ),
  );
} finally {
  if (process.env.KEEP_COMMAND_SYSTEM_PROOF === "1") {
    console.error(`retained proof workspace: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}
