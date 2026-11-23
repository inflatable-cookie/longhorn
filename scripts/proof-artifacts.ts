import { resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const members = [
  { name: "app-shell-proof", script: "scripts/verify-app-shell-proof.ts" },
  { name: "bridge-topology-conformance", script: "scripts/verify-bridge-topology-conformance.ts" },
  { name: "bridge-topology-artifacts", script: "scripts/verify-bridge-topology-artifacts.ts" },
  { name: "settings-composition-proof", script: "scripts/verify-settings-composition-proof.ts" },
  { name: "command-system-artifacts", script: "scripts/verify-command-system-artifacts.ts" },
  { name: "history-system-artifacts", script: "scripts/verify-history-system-artifacts.ts" },
  { name: "history-tree-artifacts", script: "scripts/verify-history-tree-artifacts.ts" },
  {
    name: "operation-notification-artifacts",
    script: "scripts/verify-operation-notification-artifacts.ts",
  },
  { name: "native-content-artifacts", script: "scripts/verify-native-content-artifacts.ts" },
  { name: "poodle-preview", script: "scripts/verify-poodle-preview.ts" },
  { name: "greenfield-compositions", script: "scripts/verify-greenfield-compositions.ts" },
  { name: "guide-corpus", script: "scripts/verify-guide-corpus.ts" },
  { name: "documented-commands", script: "scripts/verify-documented-commands.ts" },
  { name: "pack-typecheck", script: "scripts/verify-pack-typecheck.ts" },
] as const;

type Result = "pass" | "fail";

type Timing = {
  member: string;
  seconds: string;
  result: Result;
};

const timings: Array<Timing> = [];
let failed = false;
// Every member resolves its own storage now: checkout-level routes run through
// the switch point and pool, and each member's staged workspaces lease their own
// private final outputs under external build scratch. A `CARGO_TARGET_DIR`
// inherited from the caller would quietly move a checkout-level build off the
// pool, so it is dropped rather than forwarded.
const memberEnvironment = { ...process.env };
delete memberEnvironment.CARGO_TARGET_DIR;

for (const member of members) {
  console.log(`proof ${member.name}`);

  const startedAt = performance.now();
  let exitCode = 1;
  let stdout = "";
  try {
    const child = Bun.spawn(["bun", member.script], {
      cwd: repoRoot,
      env: memberEnvironment,
      stdout: "pipe",
      stderr: "inherit",
    });
    const [text, code] = await Promise.all([new Response(child.stdout).text(), child.exited]);
    stdout = text;
    exitCode = code;
  } catch (error) {
    console.error(`failed to start proof ${member.name}: ${String(error)}`);
  }

  timings.push({
    member: member.name,
    seconds: ((performance.now() - startedAt) / 1000).toFixed(2),
    result: exitCode === 0 ? "pass" : "fail",
  });

  if (exitCode !== 0) {
    if (stdout.trim().length > 0) {
      process.stderr.write(stdout);
      if (!stdout.endsWith("\n")) {
        process.stderr.write("\n");
      }
    }
    reportTimings();
    process.exitCode = 1;
    failed = true;
    break;
  }
}

if (!failed) {
  reportTimings();
}

function reportTimings(): void {
  console.error("proof:artifacts timings (seconds)");
  console.error("member\tseconds\tresult");
  for (const timing of timings) {
    console.error(`${timing.member}\t${timing.seconds}\t${timing.result}`);
  }
}
