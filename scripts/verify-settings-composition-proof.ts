import { join, resolve } from "node:path";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";

import {
  ACCEPT_LINKED_POODLE,
  POODLE_RELEASE,
  packLinkedPoodle,
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
  // Skipped under the exemption: `poodleRelease()` checks the registry's
  // sha512 against `bun.lock`, and a locally packed Poodle has no registry
  // entry to check against. Nothing else about the run changes.
  const poodle = ACCEPT_LINKED_POODLE
    ? { version: "0.1.0-linked", packages: [] }
    : POODLE_RELEASE;
  const typescript = await packTypescriptArtifacts(repoRoot, artifactRoot);
  const artifacts = new Map(typescript.paths);
  if (ACCEPT_LINKED_POODLE) {
    console.error(
      "settings-composition-proof: installing Poodle from the sibling checkout. " +
        "This run does NOT prove a consumer can resolve Poodle from the registry.",
    );
    for (const [name, path] of await packLinkedPoodle(artifactRoot)) {
      artifacts.set(name, path);
    }
  }
  const rust = await packAndCheckRustArtifacts(repoRoot, temporaryRoot);
  const consumers = await verifyConsumers({
    repoRoot,
    proofRoot,
    temporaryRoot,
    artifactRoot,
    artifacts,
    poodle,
  });
  console.log(
    JSON.stringify(
      {
        schema: "longhorn.settings-composition-artifact-proof.v1",
        poodleVersion: poodle.version,
        // Recorded so a green run under the exemption is never mistaken for
        // one that proved registry resolution.
        linkedPoodleAccepted: ACCEPT_LINKED_POODLE,
        rustArtifacts: rust,
        typescriptArtifacts: typescript.identities,
        consumers,
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
