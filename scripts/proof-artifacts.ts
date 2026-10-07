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

for (const member of members) {
  console.log(`proof ${member}`);

  const startedAt = performance.now();
  let exitCode = 1;
  try {
    const child = Bun.spawn(["bun", `scripts/verify-${member}.ts`], {
      cwd: repoRoot,
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
    break;
  }
}

if (timings.length === members.length) reportTimings();

function reportTimings(): void {
  console.error("proof:artifacts timings (seconds)");
  console.error("member\tseconds\tresult");
  for (const timing of timings) {
    console.error(`${timing.member}\t${timing.seconds}\t${timing.result}`);
  }
}
