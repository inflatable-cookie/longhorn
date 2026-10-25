import { resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const members = [
  "app-shell-proof",
  "bridge-topology-conformance",
  "bridge-topology-artifacts",
  "settings-composition-proof",
  "command-system-artifacts",
  "history-system-artifacts",
  "history-tree-artifacts",
  "operation-notification-artifacts",
  "native-content-artifacts",
  "poodle-preview",
  "greenfield-card125",
  "guides-card126",
  "documented-commands",
  "pack-typecheck",
] as const;

type Result = "pass" | "fail";

type Timing = {
  member: string;
  seconds: string;
  result: Result;
};

const timings: Timing[] = [];
let failed = false;
// Every member resolves its own storage now: checkout-level routes run through
// the switch point and pool, and each member's staged workspaces lease their own
// private final outputs under external build scratch. A `CARGO_TARGET_DIR`
// inherited from the caller would quietly move a checkout-level build off the
// pool, so it is dropped rather than forwarded.
const memberEnvironment = { ...process.env };
delete memberEnvironment.CARGO_TARGET_DIR;

for (const member of members) {
  console.log(`proof ${member}`);

  const startedAt = performance.now();
  let exitCode = 1;
  try {
    const child = Bun.spawn(["bun", `scripts/verify-${member}.ts`], {
      cwd: repoRoot,
      env: memberEnvironment,
      stdout: "ignore",
      stderr: "inherit",
    });
    exitCode = await child.exited;
  } catch (error) {
    console.error(`failed to start proof ${member}: ${String(error)}`);
  }

  timings.push({
    member,
    seconds: ((performance.now() - startedAt) / 1000).toFixed(2),
    result: exitCode === 0 ? "pass" : "fail",
  });

  if (exitCode !== 0) {
    reportTimings();
    process.exitCode = 1;
    failed = true;
    break;
  }
}

if (!failed) reportTimings();

function reportTimings(): void {
  console.error("proof:artifacts timings (seconds)");
  console.error("member\tseconds\tresult");
  for (const timing of timings) {
    console.error(`${timing.member}\t${timing.seconds}\t${timing.result}`);
  }
}
