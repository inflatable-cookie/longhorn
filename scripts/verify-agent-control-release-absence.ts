// Proves contract 022's compile-time opt-in for the agent control surface:
// three feature states of `longhorn-tauri-agent-control`, each with a
// positive control so a broken scan cannot pass vacuously.
//
//   neither feature — `cargo tree` shows no `longhorn-agent-control`, no
//     core rlib exists in the build, and the plugin artifact carries neither
//     the core crate's symbol prefix nor its source-path strings;
//   `agent-control` only — the server is present and `evaluate` markers are
//     absent from the core artifact; isolated tests prove the tool answers
//     typed `Unsupported`;
//   both features — the same probes FIND `evaluate`, proving the markers
//     are live and the scan can detect what it forbids.
//
// Builds run release-shaped in isolated target dirs under
// `target/agent-control-scan/` so a previous opposite-feature build can never
// leave a stale artifact for the scan to misread, and so parallel qa lanes
// never observe a half-switched `target/`. Scanning is byte-level against the
// rlib files themselves — no external binutils, matching the runner-tools
// posture that a clean runner has a Rust toolchain and nothing else.

import { readdir, readFile, rm } from "node:fs/promises";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const PLUGIN = "longhorn-tauri-agent-control";
const CORE = "longhorn-agent-control";
const SCAN_ROOT = join(repoRoot, "target", "agent-control-scan");

// A core-crate symbol reference (underscored crate name) and a core-crate
// source path (panic locations record it). Both are absent when the core
// crate is not in the graph; the hyphen-free plugin crate's own name can
// never produce either.
const CORE_MARKERS = ["longhorn_agent_control", "longhorn-agent-control/src"];
const SHIM_MARKERS = ["data-longhorn-agent-ref", "__longhornAgentControl"];
const MARKERS = [...CORE_MARKERS, ...SHIM_MARKERS];
// Present only when `agent-control-evaluate` compiles the JS escape hatch
// into server instructions (mcp.rs). Doc comments do not reach the rlib.
const EVALUATE_MARKERS = ["full code execution in the app"];

type FeatureSet = "off" | "agent-control" | "agent-control-evaluate";

type BuildResult = {
  features: FeatureSet;
  targetDir: string;
  tree: string;
  pluginRlib: string;
  coreRlib: string | null;
};

function featureArgs(features: FeatureSet): string[] {
  switch (features) {
    case "off":
      return ["--no-default-features"];
    case "agent-control":
      return ["--features", "agent-control"];
    case "agent-control-evaluate":
      return ["--features", "agent-control,agent-control-evaluate"];
  }
}

async function run(command: readonly string[], env: Record<string, string>) {
  const subprocess = Bun.spawn(command, {
    cwd: repoRoot,
    env: { ...process.env, ...env },
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    subprocess.exited,
    new Response(subprocess.stdout).text(),
    new Response(subprocess.stderr).text(),
  ]);
  if (exitCode !== 0) {
    throw new Error(`${command.join(" ")} failed\n${stdout}\n${stderr}`);
  }
  return stdout;
}

async function build(features: FeatureSet): Promise<BuildResult> {
  const args = featureArgs(features);
  const targetDir = join(SCAN_ROOT, features);
  const env = { CARGO_TARGET_DIR: targetDir };

  // Drop prior scan artifacts for the two crates under test (only) so a
  // source change since the last run cannot leave a second hash beside the
  // current one; the dependency cache stays warm.
  const depsDir = join(targetDir, "release", "deps");
  const prior = await readdir(depsDir).catch(() => [] as string[]);
  for (const entry of prior) {
    if (
      entry.startsWith("liblonghorn_agent_control-") ||
      entry.startsWith("liblonghorn_tauri_agent_control-")
    ) {
      await rm(join(depsDir, entry));
    }
  }

  const tree = await run(
    [
      "cargo",
      "tree",
      "-p",
      PLUGIN,
      "--locked",
      "-e",
      "normal",
      "--prefix",
      "none",
      ...args,
    ],
    {},
  );
  await run(
    [
      "cargo",
      "build",
      "-p",
      PLUGIN,
      "--release",
      "--locked",
      "--lib",
      ...args,
    ],
    env,
  );

  const entries = await readdir(depsDir);
  const coreRlibs = entries.filter(
    (entry) => entry.startsWith("liblonghorn_agent_control-") && entry.endsWith(".rlib"),
  );
  const pluginRlibs = entries.filter(
    (entry) =>
      entry.startsWith("liblonghorn_tauri_agent_control-") && entry.endsWith(".rlib"),
  );

  if (features === "off") {
    if (tree.includes(`${CORE} `) || tree.includes(`${CORE}@`)) {
      throw new Error(
        `feature-off dependency graph contains ${CORE} — feature unification pulled the control surface into a featureless build:\n${tree}`,
      );
    }
    if (coreRlibs.length > 0) {
      throw new Error(
        `feature-off build produced core-crate artifacts: ${coreRlibs.join(", ")}`,
      );
    }
  } else {
    if (!tree.includes(CORE)) {
      throw new Error(
        `${features} dependency graph is missing ${CORE} — the scan's positive control cannot see the surface:\n${tree}`,
      );
    }
    if (coreRlibs.length === 0) {
      throw new Error(`${features} build produced no core-crate rlib`);
    }
  }
  if (pluginRlibs.length !== 1) {
    throw new Error(
      `expected exactly one plugin rlib, found ${pluginRlibs.length}: ${pluginRlibs.join(", ")}`,
    );
  }
  return {
    features,
    targetDir,
    tree,
    pluginRlib: join(depsDir, pluginRlibs[0]!),
    coreRlib: coreRlibs.length === 0 ? null : join(depsDir, coreRlibs[0]!),
  };
}

async function markerHits(path: string, markers: readonly string[]): Promise<string[]> {
  const bytes = await readFile(path);
  return markers.filter((marker) => bytes.includes(marker));
}

async function proveServerPresent(buildResult: BuildResult): Promise<{
  pluginMarkersFound: string[];
  coreMarkersFound: string[];
}> {
  if (buildResult.coreRlib === null) {
    throw new Error(`${buildResult.features} build produced no core-crate rlib`);
  }
  const pluginHits = await markerHits(buildResult.pluginRlib, MARKERS);
  if (!pluginHits.includes(CORE_MARKERS[0]!)) {
    throw new Error(
      `${buildResult.features} release artifact does not reference ${CORE_MARKERS[0]} — the scan would pass vacuously`,
    );
  }
  const missingShim = SHIM_MARKERS.filter((marker) => !pluginHits.includes(marker));
  if (missingShim.length > 0) {
    throw new Error(
      `${buildResult.features} plugin artifact is missing shim markers: ${missingShim.join(", ")} — the gated injectable is not in the feature-on build`,
    );
  }
  const coreHits = await markerHits(buildResult.coreRlib, CORE_MARKERS);
  const missingCore = CORE_MARKERS.filter((marker) => !coreHits.includes(marker));
  if (missingCore.length > 0) {
    throw new Error(
      `${buildResult.features} core artifact is missing markers the scan forbids feature-off: ${missingCore.join(", ")} — the scan would pass vacuously`,
    );
  }
  return { pluginMarkersFound: pluginHits, coreMarkersFound: coreHits };
}

const off = await build("off");
const packaged = await build("agent-control");
const withEvaluate = await build("agent-control-evaluate");

const offHits = await markerHits(off.pluginRlib, MARKERS);
if (offHits.length > 0) {
  throw new Error(
    `feature-off release artifact carries gated surface markers: ${offHits.join(", ")}`,
  );
}

const packagedHits = await proveServerPresent(packaged);
const packagedEvaluate = await markerHits(packaged.coreRlib!, EVALUATE_MARKERS);
if (packagedEvaluate.length > 0) {
  throw new Error(
    `agent-control-only core artifact carries evaluate markers: ${packagedEvaluate.join(", ")}`,
  );
}

const evaluateHits = await proveServerPresent(withEvaluate);
const evaluatePresent = await markerHits(withEvaluate.coreRlib!, EVALUATE_MARKERS);
const missingEvaluate = EVALUATE_MARKERS.filter(
  (marker) => !evaluatePresent.includes(marker),
);
if (missingEvaluate.length > 0) {
  throw new Error(
    `agent-control-evaluate core artifact is missing evaluate markers: ${missingEvaluate.join(", ")} — the scan would pass vacuously`,
  );
}

// Isolated `-p` tests: workspace unification from the proof example would
// otherwise compile `agent-control-evaluate` into every crate test.
await run(
  [
    "cargo",
    "test",
    "-p",
    CORE,
    "--locked",
    "--test",
    "conformance",
    "evaluate_answers_typed_unsupported",
    "--",
    "--exact",
  ],
  {},
);
await run(
  [
    "cargo",
    "test",
    "-p",
    PLUGIN,
    "--features",
    "agent-control",
    "--locked",
    "--test",
    "mount",
    "evaluate_answers_typed_unsupported",
    "--",
    "--exact",
  ],
  {},
);
await run(
  [
    "cargo",
    "test",
    "-p",
    CORE,
    "--features",
    "agent-control-evaluate",
    "--locked",
    "--test",
    "conformance",
    "two_clients_interleave_without_cross_talk",
    "--",
    "--exact",
  ],
  {},
);

console.log(
  JSON.stringify(
    {
      schema: "longhorn.agent-control-release-absence.v2",
      outcome: "pass",
      defaultOff: {
        coreInGraph: false,
        coreArtifacts: 0,
        markersFound: offHits,
      },
      agentControl: {
        coreInGraph: true,
        pluginMarkersFound: packagedHits.pluginMarkersFound,
        coreMarkersFound: packagedHits.coreMarkersFound,
        evaluateMarkersFound: packagedEvaluate,
      },
      agentControlEvaluate: {
        coreInGraph: true,
        pluginMarkersFound: evaluateHits.pluginMarkersFound,
        coreMarkersFound: evaluateHits.coreMarkersFound,
        evaluateMarkersFound: evaluatePresent,
      },
      markers: MARKERS,
      evaluateMarkers: EVALUATE_MARKERS,
    },
    null,
    2,
  ),
);
