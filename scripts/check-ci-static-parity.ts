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

function escapeRegExp(text: string): string {
  return text.replaceAll(/[.*+?^${}()|[\]\\]/g, String.raw`\$&`);
}

// Text of one job under `jobs:`, from its header to the next job header.
export function parseWorkflowJob(workflow: string, job: string): string {
  const header = new RegExp(`^  ${escapeRegExp(job)}:\\s*$`, "m").exec(workflow);
  if (header === null) {
    throw new Error(`ci.yml has no ${job} job`);
  }
  const rest = workflow.slice(header.index + header[0].length);
  const next = /^ {2}[A-Za-z0-9_-]+:\s*$/m.exec(rest);
  return next === null ? rest : rest.slice(0, next.index);
}

// Distinct Effigy task names invoked in a workflow fragment. Comment lines are
// skipped, so prose such as "Effigy runs the same selectors" is not a step.
export function parseEffigyTasks(text: string): Array<string> {
  const tasks: Array<string> = [];
  for (const line of text.split("\n")) {
    if (line.trim().startsWith("#")) {
      continue;
    }
    for (const match of line.matchAll(/\beffigy\s+([A-Za-z][\w:.-]*)/g)) {
      const task = match[1] ?? "";
      if (task !== "" && !tasks.includes(task)) {
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
  const runs = parseEffigyTasks(parseWorkflowJob(inputs.workflow, CLIENTS_JOB));
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
  if (!/^\s*run:\s*effigy qa\s*$/m.test(inputs.releaseWorkflow)) {
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
