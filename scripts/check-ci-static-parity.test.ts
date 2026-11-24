import { describe, expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

import {
  CLIENTS_JOB,
  checkCiStaticParity,
  clientsParityProblems,
  parseEffigyTasks,
  parseQaTasks,
  parseWorkflowJob,
} from "./check-ci-static-parity.ts";

const repoRoot = resolve(import.meta.dir, "..");

async function realInputs() {
  const [workflow, releaseWorkflow, effigyToml] = await Promise.all([
    readFile(join(repoRoot, ".github/workflows/ci.yml"), "utf8"),
    readFile(join(repoRoot, ".github/workflows/release.yml"), "utf8"),
    readFile(join(repoRoot, "effigy.toml"), "utf8"),
  ]);
  return { workflow, releaseWorkflow, effigyToml };
}

describe("parseWorkflowJob", () => {
  const workflow = [
    "jobs:",
    "  rust:",
    "    steps:",
    "      - run: effigy lint:rust",
    "  clients:",
    "    steps:",
    "      - run: effigy check:ts",
    "  external-source:",
    "    steps:",
    "      - run: effigy release:source-consumer",
    "",
  ].join("\n");

  test("returns only the named job's body", () => {
    const body = parseWorkflowJob(workflow, CLIENTS_JOB);
    expect(body).toContain("effigy check:ts");
    expect(body).not.toContain("lint:rust");
    expect(body).not.toContain("release:source-consumer");
  });

  test("refuses a workflow with no such job", () => {
    expect(() => parseWorkflowJob(workflow, "missing")).toThrow("no missing job");
  });
});

describe("parseEffigyTasks", () => {
  test("reads task names from run steps, dedupes, and skips comments", () => {
    const text = [
      "      # Effigy runs the same selectors a local effigy qa runs",
      "      - run: effigy check:ts",
      "        run: |",
      "          effigy test:ts",
      "          effigy test:vitest",
      "      - run: effigy check:ts",
      "      - uses: inflatable-cookie/setup-effigy@abc",
    ].join("\n");
    expect(parseEffigyTasks(text)).toEqual(["check:ts", "test:ts", "test:vitest"]);
  });
});

describe("parseQaTasks", () => {
  test("reads the qa aggregate in order and stops at its closing bracket", () => {
    const toml = [
      "qa = [",
      '  { task = "check:ts" },',
      '  { task = "lint:ts" },',
      "]",
      '"after" = { task = "not-qa" }',
      "",
    ].join("\n");
    expect(parseQaTasks(toml)).toEqual(["check:ts", "lint:ts"]);
  });
});

describe("clientsParityProblems", () => {
  test("the repository's own workflow, qa list and release workflow agree", async () => {
    expect(clientsParityProblems(await realInputs())).toEqual([]);
    expect(await checkCiStaticParity()).toEqual([]);
  });

  test("a new TypeScript gate in qa that the job neither runs nor declares fails", async () => {
    const inputs = await realInputs();
    const drifted = {
      ...inputs,
      effigyToml: inputs.effigyToml.replace(
        '  { task = "check:knip" },',
        '  { task = "check:knip" },\n  { task = "check:new-ts" },',
      ),
    };
    expect(clientsParityProblems(drifted)).toEqual([
      "qa runs check:new-ts, but the clients job neither runs it nor declares it omitted",
    ]);
  });

  test("a family gate the job drops without a declaration fails", async () => {
    const inputs = await realInputs();
    const drifted = {
      ...inputs,
      workflow: inputs.workflow.replace(
        "        run: effigy check:svelte",
        "        run: echo skipped",
      ),
    };
    expect(clientsParityProblems(drifted)).toEqual([
      "qa runs check:svelte, but the clients job neither runs it nor declares it omitted",
    ]);
  });

  test("a declared omission that the job starts running fails", async () => {
    const inputs = await realInputs();
    const drifted = {
      ...inputs,
      workflow: inputs.workflow.replace(
        "        run: effigy check:bindings",
        "        run: effigy check:bindings\n      - run: effigy lint:ts",
      ),
    };
    expect(clientsParityProblems(drifted)).toEqual([
      "lint:ts is declared omitted, but the clients job runs it",
    ]);
  });

  test("a job step naming a task qa does not run fails", async () => {
    const inputs = await realInputs();
    const drifted = {
      ...inputs,
      workflow: inputs.workflow.replace(
        "        run: effigy check:bindings",
        "        run: effigy check:bindings\n      - run: effigy check:unlisted",
      ),
    };
    expect(clientsParityProblems(drifted)).toEqual([
      "the clients job runs check:unlisted, which qa does not run",
    ]);
  });

  test("release.yml that stops running effigy qa leaves the omissions uncovered", async () => {
    const inputs = await realInputs();
    const problems = clientsParityProblems({ ...inputs, releaseWorkflow: "name: release\n" });
    expect(problems).toEqual([
      "release.yml no longer runs `effigy qa`, so the omitted gates have no covering run",
    ]);
  });
});
