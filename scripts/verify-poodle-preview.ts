import { poodleRelease, poodleSveltePeerRange } from "./poodle-release.ts";

// Card 038's claim: the Poodle packages Longhorn mounts are the exact artifacts
// the lockfile records, and their Svelte compatibility has not moved.
//
// Both halves now come from the published release rather than from a sibling
// checkout's evidence file. `poodleRelease()` checks each package's sha512
// against bun.lock and against the copy installed in node_modules; the peer
// range is read off the installed adapter, so this asserts what the artifact
// declares rather than what a file alongside it claimed.
const release = poodleRelease();
const sveltePeerRange = poodleSveltePeerRange();

if (sveltePeerRange !== ">=5.56.8 <6") {
  throw new Error(
    `Poodle Svelte compatibility moved: the published adapter declares ${sveltePeerRange}`,
  );
}

console.log(
  `verified Poodle ${release.version} (${release.packages.length} published packages)`,
);
