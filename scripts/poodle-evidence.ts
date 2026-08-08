// One source of truth for which Poodle artifact set the proofs run against.
//
// Eight proof scripts used to hardcode both the evidence path and the expected
// artifact set id. Poodle mints a fresh `.artifacts/svelte-pack-install-*`
// directory on every pack rebuild, so all eight went stale together and stayed
// stale — eleven of twelve artifact proofs were failing on it.
//
// The pin now lives in exactly one place that already has to be correct: the
// root manifest's `@inflatable-cookie/poodle-core` dependency. If the packs
// Longhorn installs and the packs the proofs verify could ever disagree, the
// proofs would be verifying something no consumer resolves.
//
// The integrity claim is unchanged. Every pack on disk must match the SHA-256
// recorded for it, and the set id must equal the membership hash over
// `name:sha256` pairs. What is gone is the frozen literal, which asserted
// "this exact historical set" and could only ever rot.

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");

export interface PoodleArtifact {
  readonly name: string;
  readonly version: string;
  readonly filename: string;
  readonly bytes: number;
  readonly sha256: string;
}

export interface PoodleEvidence {
  readonly artifactSetId: string;
  readonly svelteFloor: string;
  readonly sveltePeerRange: string;
  readonly artifacts: readonly PoodleArtifact[];
}

function pinnedEvidencePath(): string {
  const override = process.env.POODLE_PREVIEW_EVIDENCE;
  if (override) return resolve(override);

  const manifest = JSON.parse(
    readFileSync(resolve(repoRoot, "package.json"), "utf8"),
  ) as {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
    overrides?: Record<string, string>;
  };
  const pin =
    manifest.devDependencies?.["@inflatable-cookie/poodle-core"] ??
    manifest.dependencies?.["@inflatable-cookie/poodle-core"] ??
    manifest.overrides?.["@inflatable-cookie/poodle-core"];
  if (!pin?.startsWith("file:")) {
    throw new Error(
      "@inflatable-cookie/poodle-core is not pinned to a packed tarball; " +
        "the proofs cannot determine which Poodle artifact set to verify",
    );
  }
  // file:../poodle/.artifacts/<set>/packs/<file>.tgz -> .../<set>/evidence.json
  const packPath = resolve(repoRoot, pin.slice("file:".length));
  return resolve(dirname(dirname(packPath)), "evidence.json");
}

let cached: (PoodleEvidence & { evidencePath: string; packDirectory: string }) | null =
  null;

export function poodleEvidence(): PoodleEvidence & {
  readonly evidencePath: string;
  readonly packDirectory: string;
} {
  if (cached) return cached;

  const evidencePath = pinnedEvidencePath();
  const evidence = JSON.parse(
    readFileSync(evidencePath, "utf8"),
  ) as PoodleEvidence;
  const packDirectory = resolve(dirname(evidencePath), "packs");

  for (const artifact of evidence.artifacts) {
    const bytes = readFileSync(resolve(packDirectory, artifact.filename));
    const digest = createHash("sha256").update(bytes).digest("hex");
    if (digest !== artifact.sha256) {
      throw new Error(
        `${artifact.name} Poodle artifact digest mismatch: expected ${artifact.sha256}, received ${digest}`,
      );
    }
  }

  const computed = createHash("sha256")
    .update(
      evidence.artifacts
        .map((artifact) => `${artifact.name}:${artifact.sha256}`)
        .join("\n"),
    )
    .digest("hex");
  if (computed !== evidence.artifactSetId) {
    throw new Error(
      `Poodle artifact membership mismatch: computed ${computed}, recorded ${evidence.artifactSetId}`,
    );
  }

  cached = { ...evidence, evidencePath, packDirectory };
  return cached;
}

/** The membership hash of the currently pinned set. */
export function poodleArtifactSet(): string {
  return poodleEvidence().artifactSetId;
}

/** Package names in the pinned set, sorted. */
export function poodlePackageNames(): readonly string[] {
  return poodleEvidence()
    .artifacts.map((artifact) => artifact.name)
    .slice()
    .sort();
}
