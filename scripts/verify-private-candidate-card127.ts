import { mkdir, mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";
import {
  longhornPackageManifests,
  poodlePackages,
  produceArtifacts,
  rustPackages,
} from "./private-candidate-card127/artifacts";
import {
  assertGraphSourcesClean,
  graphDefinitions,
  verifyConsumerGraphs,
} from "./private-candidate-card127/consumers";
import {
  candidateVersion,
  digest,
  equal,
  freezeSource,
  git,
  hash,
  poodleRoot,
  repoRoot,
  run,
  stableJson,
} from "./private-candidate-card127/support";

type PriorReceipt = {
  sources: {
    longhorn: { commit: string };
    poodle: { commit: string };
  };
};

const fixturePath = join(repoRoot, "fixtures/release/card127/private-0-1-candidate-v1.json");
const write = process.argv.includes("--write");
const prior = write ? null : JSON.parse(await readFile(fixturePath, "utf8")) as PriorReceipt;
const longhornCommit = prior?.sources.longhorn.commit ?? await git(repoRoot, ["rev-parse", "HEAD"]);
const poodleCommit = prior?.sources.poodle.commit ?? await git(poodleRoot, ["rev-parse", "HEAD"]);
const temporaryRoot = await mkdtemp(join(tmpdir(), "longhorn-card127-candidate-"));

try {
  const manifests = await longhornPackageManifests();
  const crates = await rustPackages();
  const longhornPaths = [
    "Cargo.toml",
    "Cargo.lock",
    "bun.lock",
    ...[...manifests.values()].map(({ directory }) => directory),
    ...crates.map(({ manifest_path }) => relative(repoRoot, manifest_path).replace(/\/Cargo\.toml$/, "")),
  ];
  const poodlePaths = poodlePackages.map(([, path]) => path);
  const definitions = await graphDefinitions();
  const sources = {
    longhorn: await freezeSource(repoRoot, longhornCommit, longhornPaths),
    poodle: await freezeSource(poodleRoot, poodleCommit, poodlePaths),
    consumers: Object.fromEntries(definitions
      .filter(({ repository }) => repository !== null)
      .map(({ name, sourceCommit, sourceManifests }) => [name, {
        repository: name === "bovine" ? "acowtancy/bovine-accelerator-desktop" : name,
        branch: "main",
        commit: sourceCommit,
        selectedManifests: sourceManifests,
      }])),
  };
  await assertGraphSourcesClean(definitions);
  const artifacts = await produceArtifacts(temporaryRoot, longhornCommit, poodleCommit);
  const graphs = await verifyConsumerGraphs(temporaryRoot, definitions, artifacts, manifests, longhornCommit);
  const protocols = await protocolInventory();
  const peerMatrix = [...manifests].map(([name, { manifest }]) => ({
    package: name,
    peers: Object.fromEntries(Object.entries(manifest.peerDependencies ?? {}).sort(([left], [right]) => left.localeCompare(right))),
    optionalPeers: Object.keys(manifest.peerDependenciesMeta ?? {})
      .filter((peer) => manifest.peerDependenciesMeta?.[peer]?.optional === true)
      .sort(),
  }));
  const report = {
    schema: "longhorn.private-compatibility-candidate.v1",
    outcome: "pass",
    version: candidateVersion,
    sources,
    tools: {
      rust: "1.90.0",
      rustc: (await run(["rustc", "+1.90.0", "--version"])).trim(),
      tauriRust: "2.11.5",
      bun: (await run(["bun", "--version"])).trim(),
      repositoryTypescript: (await run(["bun", "x", "tsc", "--version"])).trim().replace(/^Version /, ""),
      isolatedConsumerTypescript: "6.0.3",
      svelte: "5.56.8",
      tauriApi: "2.11.1",
    },
    artifacts: {
      longhornTypescript: { setSha256: artifacts.typescript.setSha256, packages: artifacts.typescript.identities },
      poodle: { setSha256: artifacts.poodle.setSha256, packages: artifacts.poodle.identities },
      longhornRust: { setSha256: artifacts.rust.setSha256, packages: artifacts.rust.identities },
      rustLockSha256: graphs.rustLockSha256,
      reproducibleRuns: 2,
    },
    compatibility: {
      peerMatrix,
      protocols,
      protocolNegotiation: "exact-v1",
      platform: {
        pureMechanisms: "platform-neutral-at-injected-boundary",
        tauriWindowing: { macos: "packaged", windows: "unproved", linux: "unproved" },
        childView: { macos: "packaged", windows: "unproved", linux: "unproved" },
        isolatedWindow: { macos: "packaged", windows: "unsupported", linux: "unsupported" },
        backingSurface: { macos: "packaged-live-scale-unmet", windows: "unsupported", linux: "unsupported" },
      },
    },
    graphs: graphs.proofs,
    releaseGates: {
      readOnlyOnly: true,
      packageNamesAndRegistryOwnership: "unresolved",
      normalizedCargoPackages: "deferred-until-public-release-lane",
    },
    audits: {
      coordinatedVersion: true,
      generatedBindingDrift: false,
      reproduciblePackageBytes: true,
      exactSourceMapping: true,
      isolatedConsumerInstalls: true,
      siblingWorkspaceResolution: false,
      optionalEdgesExact: true,
      consumerRepositoryWrites: false,
      packageManagerPublication: false,
      gitTags: false,
      hostedReleases: false,
    },
  };

  if (write) {
    await mkdir(join(repoRoot, "fixtures/release/card127"), { recursive: true });
    await writeFile(fixturePath, stableJson(report));
  } else {
    const expected = JSON.parse(await readFile(fixturePath, "utf8"));
    equal(report, expected, "private candidate receipt");
  }
  console.log(stableJson({
    schema: "longhorn.private-compatibility-candidate-proof.v1",
    outcome: "pass",
    mode: write ? "write" : "check",
    version: candidateVersion,
    source: longhornCommit,
    typescriptPackages: artifacts.typescript.identities.length,
    rustPackages: artifacts.rust.identities.length,
    poodlePackages: artifacts.poodle.identities.length,
    consumerGraphs: graphs.proofs.length,
    candidateSha256: hash(stableJson(report)),
    packageManagerPublication: false,
  }));
} finally {
  if (process.env.KEEP_PRIVATE_CANDIDATE_CARD127 === "1") {
    console.error(`retained Card 127 candidate: ${temporaryRoot}`);
  } else {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

async function protocolInventory() {
  const paths = (await readdir(join(repoRoot, "fixtures"), { recursive: true }))
    .filter((path) => path.endsWith("protocol-v1.json"))
    .sort();
  return Promise.all(paths.map(async (path) => ({
    path: `fixtures/${path}`,
    sha256: await digest(join(repoRoot, "fixtures", path)),
    supported: "exact-v1",
  })));
}
