import { resolve } from "node:path";
import type { ProofMember } from "./proof-member-run.ts";
import { failedMembers, formatTimings, runProofMembers } from "./proof-member-run.ts";

const repoRoot = resolve(import.meta.dir, "..");
const members: ReadonlyArray<ProofMember> = [
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
];

// Every member resolves its own storage now: checkout-level routes run through
// the switch point and pool, and each member's staged workspaces lease their own
// private final outputs under external build scratch. A `CARGO_TARGET_DIR`
// inherited from the caller would quietly move a checkout-level build off the
// pool, so it is dropped rather than forwarded.
const memberEnvironment = { ...process.env };
delete memberEnvironment.CARGO_TARGET_DIR;

const timings = await runProofMembers(members, async (member) => {
  console.log(`proof ${member.name}`);
  const child = Bun.spawn(["bun", member.script], {
    cwd: repoRoot,
    env: memberEnvironment,
    stdout: "pipe",
    stderr: "inherit",
  });
  const [stdout, exitCode] = await Promise.all([new Response(child.stdout).text(), child.exited]);
  return { exitCode, stdout };
});

// A failing member is reported with its captured output, after every member
// has run, so one broken proof does not hide the verdict of the others.
for (const timing of timings) {
  if (timing.result === "fail" && timing.stdout.trim().length > 0) {
    process.stderr.write(timing.stdout.endsWith("\n") ? timing.stdout : `${timing.stdout}\n`);
  }
}
for (const line of formatTimings(timings)) {
  console.error(line);
}

const failures = failedMembers(timings);
if (failures.length > 0) {
  console.error(`proof:artifacts failed: ${failures.join(", ")}`);
  process.exitCode = 1;
}
