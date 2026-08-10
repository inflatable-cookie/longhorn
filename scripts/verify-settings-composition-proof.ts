import { join, resolve } from "node:path";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";

import {
  POODLE_RELEASE,
  packAndCheckRustArtifacts,
  packTypescriptArtifacts,
} from "./settings-composition-proof/artifacts.ts";
import { verifyConsumers } from "./settings-composition-proof/consumers.ts";

const repoRoot = resolve(import.meta.dir, "..");
const proofRoot = join(repoRoot, "examples/settings-composition-proof");
const temporaryRoot = await mkdtemp(
  join(tmpdir(), "longhorn-settings-proof-"),
);
const artifactRoot = join(temporaryRoot, "typescript-artifacts");
await mkdir(artifactRoot);

try {
  const poodle = POODLE_RELEASE;
  const typescript = await packTypescriptArtifacts(repoRoot, artifactRoot);
  const rust = await packAndCheckRustArtifacts(repoRoot, temporaryRoot);
  const consumers = await verifyConsumers({
    repoRoot,
    proofRoot,
    temporaryRoot,
    artifactRoot,
    artifacts: typescript.paths,
    poodle,
  });
  console.log(
    JSON.stringify(
      {
        schema: "longhorn.settings-composition-artifact-proof.v1",
        poodleVersion: POODLE_RELEASE.version,
        rustArtifacts: rust,
        typescriptArtifacts: typescript.identities,
        consumers,
        hostForms: ["modal", "window", "panel"],
        duplicatePeerRuntime: false,
        upwardOptionalEdge: false,
        productAuthorityMoved: false,
        crossDomainAtomicityClaimed: false,
      },
      null,
      2,
    ),
  );
} finally {
  if (process.env.KEEP_SETTINGS_PROOF === "1") {
    console.error(`retained proof workspace: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}
