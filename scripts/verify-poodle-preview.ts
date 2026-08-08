import { poodleEvidence } from "./poodle-evidence.ts";

// Card 038's claim: the Poodle preview artifacts Longhorn mounts are the exact
// tarballs their evidence records, and their Svelte compatibility has not
// moved. Which set that is comes from the root manifest pin — see
// ./poodle-evidence.ts for why it is no longer a literal here.
const evidence = poodleEvidence();

if (
  evidence.svelteFloor !== "5.38.6" ||
  evidence.sveltePeerRange !== ">=5.38.6 <6"
) {
  throw new Error("Poodle Svelte compatibility evidence changed");
}

console.log(
  `verified Poodle artifact set ${evidence.artifactSetId} (${evidence.artifacts.length} tarballs)`,
);
