import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";

const expectedSetId =
  "39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74";
const defaultEvidencePath = resolve(
  import.meta.dir,
  "../../poodle/.artifacts/g12.016-A698XB/evidence.json",
);
const evidencePath = resolve(
  process.env.POODLE_CARD_038_EVIDENCE ?? defaultEvidencePath,
);
const evidence = JSON.parse(readFileSync(evidencePath, "utf8")) as Evidence;

if (evidence.artifactSetId !== expectedSetId) {
  throw new Error(
    `Poodle artifact set mismatch: expected ${expectedSetId}, received ${evidence.artifactSetId}`,
  );
}
if (
  evidence.svelteFloor !== "5.38.6" ||
  evidence.sveltePeerRange !== ">=5.38.6 <6"
) {
  throw new Error("Poodle Svelte compatibility evidence changed");
}

const artifactDirectory = resolve(dirname(evidencePath), "packs");
for (const artifact of evidence.artifacts) {
  const bytes = readFileSync(resolve(artifactDirectory, artifact.filename));
  const digest = createHash("sha256").update(bytes).digest("hex");
  if (digest !== artifact.sha256) {
    throw new Error(
      `${artifact.name} digest mismatch: expected ${artifact.sha256}, received ${digest}`,
    );
  }
}

const computedSetId = createHash("sha256")
  .update(
    evidence.artifacts
      .map((artifact) => `${artifact.name}:${artifact.sha256}`)
      .join("\n"),
  )
  .digest("hex");
if (computedSetId !== expectedSetId) {
  throw new Error(
    `Poodle artifact membership mismatch: computed ${computedSetId}`,
  );
}

console.log(
  `verified Poodle artifact set ${expectedSetId} (${evidence.artifacts.length} tarballs)`,
);

interface Evidence {
  readonly artifactSetId: string;
  readonly svelteFloor: string;
  readonly sveltePeerRange: string;
  readonly artifacts: readonly {
    readonly name: string;
    readonly filename: string;
    readonly sha256: string;
  }[];
}
