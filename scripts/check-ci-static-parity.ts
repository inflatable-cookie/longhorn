// Keep the hosted clients job honest about which static gates it runs.
//
// ci.yml's header says Effigy runs the same selectors a local `effigy qa` runs.
// The clients job runs only part of qa's TypeScript and Svelte family: lint:ts,
// fmt:ts and check:knip are not in it. This check records that subset here,
// fails on drift in either direction, and requires each omitted gate to be
// run by release.yml's `effigy qa` step. Closing the gap means editing
// .github/workflows/ci.yml, which needs Tom's approval, so the gap stays
// declared below until that edit lands.

import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");

export const CLIENTS_JOB = "clients";

// qa members that a TypeScript or Svelte job would run. A new member matching
// this must be run by the clients job or declared in CLIENTS_JOB_OMISSIONS.
export const TYPESCRIPT_FAMILY = /(?:^|[:-])(?:ts|svelte|vitest|knip|bindings)$/;

// Family gates the clients job does not run, each with the run that covers it.
export const CLIENTS_JOB_OMISSIONS = {
  "lint:ts": "effigy qa in release.yml",
  "fmt:ts": "effigy qa in release.yml",
  "check:knip": "effigy qa in release.yml",
} as const;

export type ParityInputs = {
  readonly workflow: string;
  readonly releaseWorkflow: string;
  readonly effigyToml: string;
};

export type WorkflowStep = { readonly if?: unknown; readonly run?: unknown };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

// The steps of one job, read through a YAML parser so comments, names and
// `uses:` entries are structure rather than text. A job-level `if` may skip
// every step, so a guarded job yields no steps and nothing counts as run.
export function jobSteps(workflow: string, job: string): Array<WorkflowStep> {
  const parsed: unknown = Bun.YAML.parse(workflow);
  const jobs = isRecord(parsed) ? parsed["jobs"] : undefined;
  const target = isRecord(jobs) ? jobs[job] : undefined;
  if (!isRecord(target) || !Array.isArray(target["steps"])) {
    throw new Error(`ci.yml has no ${job} job with a steps list`);
  }
  if (target["if"] !== undefined) {
    return [];
  }
  return target["steps"].filter(isRecord);
}

// Shell blocks whose bodies may not run. Each opener is closed by its closer.
const BLOCK_OPENERS: ReadonlyArray<{ readonly opener: RegExp; readonly closer: RegExp }> = [
  { opener: /^if\b/, closer: /^fi\b/ },
  { opener: /^(?:for|while|until)\b/, closer: /^done\b/ },
  { opener: /^case\b/, closer: /^esac\b/ },
];

// Effigy tasks a shell script certainly runs: command lines at the top level,
// before any `exit`, and outside every conditional or loop body. A line starting
// with `echo`, `false &&`, a comment, or anything else is not an invocation. A
// single-line block such as `if x; then effigy y; fi` leaves the scan inside a
// block, so later lines stop counting. Under-counting fails the check loudly;
// over-counting would accept a step that never runs.
function executedInScript(script: string): Array<string> {
  const tasks: Array<string> = [];
  let depth = 0;
  for (const raw of script.split("\n")) {
    const line = raw.trim();
    if (line === "" || line.startsWith("#")) {
      continue;
    }
    if (BLOCK_OPENERS.some(({ closer }) => closer.test(line))) {
      depth = Math.max(0, depth - 1);
      continue;
    }
    if (BLOCK_OPENERS.some(({ opener }) => opener.test(line))) {
      depth += 1;
      continue;
    }
    if (depth > 0) {
      continue;
    }
    if (/^exit\b/.test(line)) {
      break;
    }
    const match = /^effigy\s+([A-Za-z][\w:.-]*)/.exec(line);
    const task = match?.[1];
    if (task !== undefined) {
      tasks.push(task);
    }
  }
  return tasks;
}

// Distinct Effigy tasks that the given steps certainly run. A step with an `if`
// may be skipped, so it never counts as coverage.
export function executedEffigyTasks(steps: ReadonlyArray<WorkflowStep>): Array<string> {
  const tasks: Array<string> = [];
  for (const step of steps) {
    if (step.if !== undefined || typeof step.run !== "string") {
      continue;
    }
    for (const task of executedInScript(step.run)) {
      if (!tasks.includes(task)) {
        tasks.push(task);
      }
    }
  }
  return tasks;
}

// Tasks that any job of a workflow certainly runs, with the same rules as the
// clients job. A job without a steps list (a reusable-workflow call) runs none
// of this file's steps.
export function executedTasksAcrossJobs(workflow: string): Array<string> {
  const parsed: unknown = Bun.YAML.parse(workflow);
  const jobs = isRecord(parsed) ? parsed["jobs"] : undefined;
  if (!isRecord(jobs)) {
    return [];
  }
  const tasks: Array<string> = [];
  for (const name of Object.keys(jobs)) {
    const target = jobs[name];
    if (!isRecord(target) || !Array.isArray(target["steps"])) {
      continue;
    }
    for (const task of executedEffigyTasks(jobSteps(workflow, name))) {
      if (!tasks.includes(task)) {
        tasks.push(task);
      }
    }
  }
  return tasks;
}

// Task names of the `qa = [ ... ]` aggregate in effigy.toml, in order.
export function parseQaTasks(effigyToml: string): Array<string> {
  const header = /^qa = \[\s*$/m.exec(effigyToml);
  if (header === null) {
    throw new Error("effigy.toml has no qa aggregate");
  }
  const rest = effigyToml.slice(header.index + header[0].length);
  const end = rest.search(/^\]\s*$/m);
  const body = end === -1 ? rest : rest.slice(0, end);
  const tasks: Array<string> = [];
  for (const match of body.matchAll(/task = "([^"]+)"/g)) {
    const task = match[1] ?? "";
    if (task !== "") {
      tasks.push(task);
    }
  }
  return tasks;
}

export function clientsParityProblems(inputs: ParityInputs): Array<string> {
  const runs = executedEffigyTasks(jobSteps(inputs.workflow, CLIENTS_JOB));
  const qa = parseQaTasks(inputs.effigyToml);
  const family = qa.filter((task) => TYPESCRIPT_FAMILY.test(task));
  const problems: Array<string> = [];

  for (const task of family) {
    if (!runs.includes(task) && !Object.hasOwn(CLIENTS_JOB_OMISSIONS, task)) {
      problems.push(
        `qa runs ${task}, but the ${CLIENTS_JOB} job neither runs it nor declares it omitted`,
      );
    }
  }
  for (const task of runs) {
    if (!qa.includes(task)) {
      problems.push(`the ${CLIENTS_JOB} job runs ${task}, which qa does not run`);
    }
  }
  for (const task of Object.keys(CLIENTS_JOB_OMISSIONS)) {
    if (runs.includes(task)) {
      problems.push(`${task} is declared omitted, but the ${CLIENTS_JOB} job runs it`);
    }
    if (!family.includes(task)) {
      problems.push(`${task} is declared omitted, but qa no longer runs it`);
    }
  }
  if (!executedTasksAcrossJobs(inputs.releaseWorkflow).includes("qa")) {
    problems.push(
      "release.yml no longer runs `effigy qa`, so the omitted gates have no covering run",
    );
  }
  return problems;
}

export async function checkCiStaticParity(root: string = repoRoot): Promise<Array<string>> {
  const [workflow, releaseWorkflow, effigyToml] = await Promise.all([
    readFile(join(root, ".github/workflows/ci.yml"), "utf8"),
    readFile(join(root, ".github/workflows/release.yml"), "utf8"),
    readFile(join(root, "effigy.toml"), "utf8"),
  ]);
  return clientsParityProblems({ workflow, releaseWorkflow, effigyToml });
}

if (import.meta.main) {
  const problems = await checkCiStaticParity();
  if (problems.length > 0) {
    for (const problem of problems) {
      console.error(`check:ci-static-parity: ${problem}`);
    }
    process.exit(1);
  }
  console.log(
    `check:ci-static-parity: the ${CLIENTS_JOB} job matches the declared TypeScript subset`,
  );
}
