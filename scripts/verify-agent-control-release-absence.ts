// Proves contract 022's total-exclusion claim for the agent control surface:
// a release build of `longhorn-tauri-agent-control` without the `dev` feature
// contains no server, route, token, or discovery code — and does not even
// pull the core crate into the dependency graph (feature unification pulling
// it in is a stop condition, not a footnote).
//
// Both directions are proven, each with a positive control so a broken scan
// cannot pass vacuously:
//
//   feature off — `cargo tree` shows no `longhorn-agent-control`, no core
//     rlib exists in the build, and the plugin artifact carries neither the
//     core crate's symbol prefix nor its source-path strings;
//   feature on — the same probes must FIND the surface, proving the markers
//     are live and the scan can detect what it forbids.
//
// Builds run release-shaped in isolated target dirs under
// `target/agent-control-scan/` so a previous opposite-feature build can never
// leave a stale artifact for the scan to misread, and so parallel qa lanes
// never observe a half-switched `target/`. Scanning is byte-level against the
// rlib files themselves — no external binutils, matching the runner-tools
// posture that a clean runner has a Rust toolchain and nothing else.

import { readdir, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const PLUGIN = "longhorn-tauri-agent-control";
const CORE = "longhorn-agent-control";
const SCAN_ROOT = join(repoRoot, "target", "agent-control-scan");

// A core-crate symbol reference (underscored crate name) and a core-crate
// source path (panic locations record it). Both are absent when the core
// crate is not in the graph; the hyphen-free plugin crate's own name can
// never produce either.
const MARKERS = ["longhorn_agent_control", "longhorn-agent-control/src"];

type BuildResult = {
  targetDir: string;
  tree: string;
  pluginRlib: string;
  coreRlib: string | null;
};

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

async function build(features: "on" | "off"): Promise<BuildResult> {
  const featureArgs =
    features === "on" ? ["--features", "dev"] : ["--no-default-features"];
  const targetDir = join(SCAN_ROOT, features);
  const env = { CARGO_TARGET_DIR: targetDir };

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
      ...featureArgs,
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
      ...featureArgs,
    ],
    env,
  );

  const depsDir = join(targetDir, "release", "deps");
  const entries = await readdir(depsDir);
  const coreRlibs = entries.filter(
    (entry) => entry.startsWith("liblonghorn_agent_control-") && entry.endsWith(".rlib"),
  );
  const pluginRlibs = entries.filter(
    (entry) => entry.startsWith("liblonghorn_tauri_agent_control-") && entry.endsWith(".rlib"),
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
        `feature-on dependency graph is missing ${CORE} — the scan's positive control cannot see the surface:\n${tree}`,
      );
    }
    if (coreRlibs.length === 0) {
      throw new Error("feature-on build produced no core-crate rlib");
    }
  }
  if (pluginRlibs.length !== 1) {
    throw new Error(
      `expected exactly one plugin rlib, found ${pluginRlibs.length}: ${pluginRlibs.join(", ")}`,
    );
  }
  return {
    targetDir,
    tree,
    pluginRlib: join(depsDir, pluginRlibs[0]!),
    coreRlib: coreRlibs.length === 0 ? null : join(depsDir, coreRlibs[0]!),
  };
}

async function markerHits(path: string): Promise<string[]> {
  const bytes = await readFile(path);
  return MARKERS.filter((marker) => bytes.includes(marker));
}

const off = await build("off");
const on = await build("on");

// The artifact a featureless build ships must carry no reference to the
// gated surface.
const offHits = await markerHits(off.pluginRlib);
if (offHits.length > 0) {
  throw new Error(
    `feature-off release artifact carries gated surface markers: ${offHits.join(", ")}`,
  );
}
// Positive control: the feature-on plugin references the core crate, and
// the core artifact it links carries both markers — the scan can detect
// everything it forbids.
if (on.coreRlib === null) {
  throw new Error("feature-on build produced no core-crate rlib");
}
const onHits = await markerHits(on.pluginRlib);
if (!onHits.includes(MARKERS[0]!)) {
  throw new Error(
    `feature-on release artifact does not reference ${MARKERS[0]} — the scan would pass vacuously`,
  );
}
const coreHits = await markerHits(on.coreRlib);
const missingCore = MARKERS.filter((marker) => !coreHits.includes(marker));
if (missingCore.length > 0) {
  throw new Error(
    `feature-on core artifact is missing markers the scan forbids feature-off: ${missingCore.join(", ")} — the scan would pass vacuously`,
  );
}

console.log(
  JSON.stringify(
    {
      schema: "longhorn.agent-control-release-absence.v1",
      outcome: "pass",
      featureOff: {
        coreInGraph: false,
        coreArtifacts: 0,
        markersFound: offHits,
      },
      featureOn: {
        coreInGraph: true,
        pluginMarkersFound: onHits,
        coreMarkersFound: coreHits,
      },
      markers: MARKERS,
    },
    null,
    2,
  ),
);
