import { parseWorkflowSteps } from "./check-release-runner-tools.ts";

const REQUIRED_JOBS = ["rust", "rust-floor"] as const;
const BUN_ACTION = "oven-sh/setup-bun";

export function checkCiRunnerTools(workflow: string): string[] {
  const steps = parseWorkflowSteps(workflow);
  const clients = steps.filter((step) => step.job === "clients");
  const clientsBunIndex = clients.findIndex((step) => step.uses?.startsWith(`${BUN_ACTION}@`));
  const clientsEffigyIndex = clients.findIndex((step) => runsEffigy(step.run));
  const clientsBun = clients[clientsBunIndex];
  const failures: string[] = [];

  if (
    !clientsBun ||
    clientsBun.ifCondition !== null ||
    clientsBunIndex >= clientsEffigyIndex ||
    !isPinnedBunAction(clientsBun.uses) ||
    clientsBun.bunVersion === null
  ) {
    return ["clients must install SHA-pinned Bun before its first Effigy step"];
  }

  for (const job of REQUIRED_JOBS) {
    const jobSteps = steps.filter((step) => step.job === job);
    const bunIndex = jobSteps.findIndex((step) => step.uses?.startsWith(`${BUN_ACTION}@`));
    const effigyIndex = jobSteps.findIndex((step) => runsEffigy(step.run));
    const bun = jobSteps[bunIndex];

    if (!bun || bun.ifCondition !== null || bunIndex >= effigyIndex) {
      failures.push(`${job} must install Bun before its first Effigy step`);
      continue;
    }
    if (bun.uses !== clientsBun.uses || bun.bunVersion !== clientsBun.bunVersion) {
      failures.push(`${job} must use the same pinned Bun action and version as clients`);
    }
  }

  return failures;
}

function isPinnedBunAction(uses: string | null): boolean {
  return uses !== null && new RegExp(`^${BUN_ACTION}@[0-9a-f]{40}$`).test(uses);
}

function runsEffigy(run: string | null): boolean {
  return run !== null && /(^|\s)effigy(?:\s|$)/m.test(run);
}
