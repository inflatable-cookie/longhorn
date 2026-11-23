// Proves contract 023's independent default/release graph. The production
// dispatcher must not import or embed contract 022's developer app-control
// surface. Exact public symbols are used instead of broad words such as
// "input" or "command", which are legitimate generic vocabulary elsewhere.

import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

import { run } from "./proof-support.ts";
import {
  libraryArchives,
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
    repoRoot,
  );
  if (tree.includes(DEV_CONTROL) || tree.includes("longhorn_agent_control")) {
    throw new Error(`${DISPATCH} imports contract 022's ${DEV_CONTROL} graph`);
  }

  // The archive path is the one this build reported, so the scan never assumes
  // a Cargo output layout. The build runs in one private target inside an
  // `oxide exec` lease, from the repository root Cargo resolves against.
  const built = await run(
    rustExec(
      privateCargoCommand(scanRoot, [
        "cargo",
        "build",
        "-p",
        DISPATCH,
        "--release",
        "--locked",
        "--no-default-features",
        "--lib",
        "--message-format=json",
      ]),
      { manifestPath: join(repoRoot, "Cargo.toml"), profile: "release" },
    ),
    repoRoot,
  );

  const archives = libraryArchives(built, DISPATCH);
  if (archives.length !== 1) {
    throw new Error(`expected one dispatcher rlib, found ${archives.length}`);
  }
  const bytes = await readFile(archives[0]!);
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
} finally {
  await scratch.dispose();
}
