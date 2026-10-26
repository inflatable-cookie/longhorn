// Proves contract 023's independent default/release graph. The production
// dispatcher must not import or embed contract 022's developer app-control
// surface. Exact public symbols are used instead of broad words such as
// "input" or "command", which are legitimate generic vocabulary elsewhere.

import { readdir, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

import {
  privateCargoCommand,
  rustCommand,
  rustExec,
  taskScratch,
} from "./rust-command.ts";

const repoRoot = resolve(import.meta.dir, "..");
const DISPATCH = "longhorn-agent-tool-dispatch";
const DEV_CONTROL = "longhorn-agent-control";
const scratch = await taskScratch("longhorn-agent-tool-dispatch-scan");
const scanRoot = scratch.root;
const forbidden = [
  "longhorn_agent_control",
  "ControlHandler",
  "EvaluateRequest",
  "TypeRequest",
  "PressRequest",
  "ScreenshotRequest",
  "CommandRequest",
  "serve_control_surface",
];

try {
  const tree = await run(
    rustCommand([
      "tree",
      "-p",
      DISPATCH,
      "--locked",
      "-e",
      "normal",
      "--prefix",
      "none",
    ]),
    {},
  );
  if (tree.includes(DEV_CONTROL) || tree.includes("longhorn_agent_control")) {
    throw new Error(`${DISPATCH} imports contract 022's ${DEV_CONTROL} graph`);
  }

  // The scan reads Cargo's own `release/deps` layout, so this one build pins both
  // output roots to the scratch directory inside an `oxide exec` lease; the
  // routing table in AGENTS.md names it as the intentional private-layout
  // exception whose intermediates do not pool.
  await run(
    rustExec(
      privateCargoCommand(
        scanRoot,
        [
          "cargo",
          "build",
          "-p",
          DISPATCH,
          "--release",
          "--locked",
          "--no-default-features",
          "--lib",
        ],
        { buildDir: true },
      ),
      { manifestPath: join(repoRoot, "Cargo.toml"), profile: "release" },
    ),
    {},
  );

  const dependencies = join(scanRoot, "release", "deps");
  const artifacts = (await readdir(dependencies)).filter(
    (entry) => entry.startsWith("liblonghorn_agent_tool_dispatch-") && entry.endsWith(".rlib"),
  );
  if (artifacts.length !== 1) {
    throw new Error(`expected one dispatcher rlib, found ${artifacts.length}`);
  }
  const bytes = await readFile(join(dependencies, artifacts[0]!));
  const hits = forbidden.filter((marker) => bytes.includes(marker));
  if (hits.length > 0) {
    throw new Error(`release dispatcher contains contract 022 markers: ${hits.join(", ")}`);
  }

  const positiveControl = await readFile(
    join(repoRoot, "crates", DEV_CONTROL, "src", "lib.rs"),
    "utf8",
  );
  const positiveMarkers = ["ControlHandler", "EvaluateRequest", "CommandRequest"];
  const missingPositive = positiveMarkers.filter((marker) => !positiveControl.includes(marker));
  if (missingPositive.length > 0) {
    throw new Error(`contract 022 positive control is stale: ${missingPositive.join(", ")}`);
  }

  console.log(
    JSON.stringify(
      {
        schema: "longhorn.agent-tool-dispatch-release-absence.v1",
        outcome: "pass",
        dispatchCrate: DISPATCH,
        contract022InGraph: false,
        releaseMarkersFound: hits,
        forbidden,
        positiveControl: positiveMarkers,
      },
      null,
      2,
    ),
  );

  async function run(
    command: string[],
    environment: Record<string, string> = {},
  ): Promise<string> {
    const subprocess = Bun.spawn(command, {
      cwd: repoRoot,
      env: { ...process.env, ...environment },
      stdout: "pipe",
      stderr: "pipe",
    });
    const [stdout, stderr, exitCode] = await Promise.all([
      new Response(subprocess.stdout).text(),
      new Response(subprocess.stderr).text(),
      subprocess.exited,
    ]);
    if (exitCode !== 0) {
      throw new Error(`${command.join(" ")} failed\n${stdout}\n${stderr}`);
    }
    return stdout;
  }
} finally {
  await scratch.dispose();
}
