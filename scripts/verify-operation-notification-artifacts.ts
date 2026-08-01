import { join, resolve } from "node:path";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";

import {
  POODLE_ARTIFACT_SET,
  packAndRunRustArtifacts,
  packTypescriptArtifacts,
  readPoodleEvidence,
} from "./operation-notification-artifact-proof/artifacts.ts";
import { verifyConsumers } from "./operation-notification-artifact-proof/consumers.ts";
import { containsKey, run } from "./operation-notification-artifact-proof/shared.ts";

const repoRoot = resolve(import.meta.dir, "..");
const proofRoot = join(repoRoot, "examples", "operation-notification-proof");
const temporaryRoot = await mkdtemp(
  join(tmpdir(), "longhorn-operation-notification-proof-"),
);
const artifactRoot = join(temporaryRoot, "typescript-artifacts");
await mkdir(artifactRoot);

try {
  for (const domain of ["operation", "notifications"]) {
    await run(
      ["cargo", "run", "-p", "longhorn-bindings", "--", domain, "check"],
      repoRoot,
    );
  }
  const poodle = await readPoodleEvidence();
  const typescript = await packTypescriptArtifacts(repoRoot, artifactRoot);
  const rust = await packAndRunRustArtifacts(repoRoot, proofRoot, temporaryRoot);
  const consumers = await verifyConsumers({
    repoRoot,
    proofRoot,
    temporaryRoot,
    artifacts: typescript.paths,
    poodle,
    nativeTraces: rust.nativeTraces,
  });

  for (const key of ["productPayload", "executorPayload", "queuePayload"]) {
    if (containsKey(rust.nativeTraces, key)) {
      throw new Error(`artifact proof contains forbidden product key ${key}`);
    }
  }

  console.log(
    JSON.stringify(
      {
        schema: "longhorn.operation-notification-artifact-proof.v1",
        poodleArtifactSet: POODLE_ARTIFACT_SET,
        rustArtifacts: rust.identities,
        rustPackaging: rust.packaging,
        rustGraphs: rust.graphs,
        nativeTraces: rust.nativeTraces,
        typescriptArtifacts: typescript.identities,
        consumers,
        audits: {
          generatedBindingsDrift: false,
          nativeRendererParity: true,
          optionalEdgesProved: true,
          peerRuntimeUnique: true,
          capabilitiesExact: true,
          productPayloadAbsent: true,
          authorityRemainsConsumerOwned: true,
          operationNotificationDependencyOneWay: true,
          operationExecutorConsumerOwned: true,
          notificationActionAdmissionFresh: true,
          retainedTruthSurvivesToastExpiryAndRemount: true,
          multiWindowSessionsIsolated: true,
          teardownReleasesListeners: true,
          publicPoodleOnly: true,
          donorRepositoriesWritten: false,
        },
      },
      null,
      2,
    ),
  );
} finally {
  if (process.env.KEEP_OPERATION_NOTIFICATION_PROOF === "1") {
    console.error(`retained proof workspace: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}
