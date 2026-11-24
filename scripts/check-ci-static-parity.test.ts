import { describe, expect, test } from "bun:test";
import { readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

import {
  CLIENTS_JOB,
  checkCiStaticParity,
  clientsParityProblems,
  executedEffigyTasks,
  jobSteps,
  parseQaTasks,
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

// A workflow whose clients job has the given step YAML (already indented six spaces).
function clientsWorkflow(steps: string): string {
  return [
    "jobs:",
    "  rust:",
    "    steps:",
    "      - run: effigy lint:rust",
    "  clients:",
    "    steps:",
    steps,
    "",
  ].join("\n");
}

function executedIn(steps: string): Array<string> {
  return executedEffigyTasks(jobSteps(clientsWorkflow(steps), CLIENTS_JOB));
}

describe("jobSteps", () => {
  test("returns only the named job's steps", () => {
    const steps = jobSteps(clientsWorkflow("      - run: effigy check:ts"), CLIENTS_JOB);
    expect(steps).toEqual([{ run: "effigy check:ts" }]);
  });

  test("a job-level if may skip every step, so the job runs none of them", () => {
    const guarded = [
      "jobs:",
      "  clients:",
      "    if: false",
      "    steps:",
      "      - run: effigy check:ts",
      "",
    ].join("\n");
    expect(jobSteps(guarded, CLIENTS_JOB)).toEqual([]);
  });

  test("refuses a workflow with no such job", () => {
    expect(() => jobSteps(clientsWorkflow("      - run: true"), "missing")).toThrow("missing job");
  });
});

describe("executedEffigyTasks", () => {
  test("counts unconditional run steps, plain and block scalar, deduplicated", () => {
    expect(
      executedIn(
        [
          "      - run: effigy check:ts",
          "      - name: tests",
          "        run: |",
          "          effigy test:ts",
          "          effigy test:vitest",
          "      - run: effigy check:ts",
        ].join("\n"),
      ),
    ).toEqual(["check:ts", "test:ts", "test:vitest"]);
  });

  test("a step guarded by if may never run, so it does not count", () => {
    expect(executedIn("      - if: false\n        run: effigy check:svelte")).toEqual([]);
  });

  test("echo, a step name, and a YAML comment do not run anything", () => {
    expect(
      executedIn(
        [
          "      - run: echo effigy check:svelte",
          "      - name: effigy check:svelte",
          "        run: true",
          "      # - run: effigy check:bindings",
        ].join("\n"),
      ),
    ).toEqual([]);
  });

  test("a step that only uses an action does not count", () => {
    expect(
      executedIn(
        "      - uses: inflatable-cookie/setup-effigy@abc\n        with:\n          version: '0.14.1'",
      ),
    ).toEqual([]);
  });

  test("lines after a top-level exit never run", () => {
    expect(executedIn("      - run: |\n          exit 0\n          effigy check:svelte")).toEqual(
      [],
    );
  });

  test("lines inside an if block may not run, but later top-level lines do", () => {
    expect(
      executedIn(
        [
          "      - run: |",
          '          if [ -n "$X" ]; then',
          "            effigy check:svelte",
          "          fi",
          "          effigy check:bindings",
        ].join("\n"),
      ),
    ).toEqual(["check:bindings"]);
  });

  test("a command behind a leading false or true is not counted", () => {
    expect(executedIn("      - run: false && effigy check:svelte")).toEqual([]);
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

  test("a family gate made conditional in the workflow does not count as run", async () => {
    const inputs = await realInputs();
    const drifted = {
      ...inputs,
      workflow: inputs.workflow.replace(
        "        run: effigy check:svelte",
        "        if: false\n        run: effigy check:svelte",
      ),
    };
    expect(clientsParityProblems(drifted)).toEqual([
      "qa runs check:svelte, but the clients job neither runs it nor declares it omitted",
    ]);
  });

  test("a family gate named in an echo line does not count as run", async () => {
    const inputs = await realInputs();
    const drifted = {
      ...inputs,
      workflow: inputs.workflow.replace(
        "        run: effigy check:svelte",
        "        run: echo effigy check:svelte",
      ),
    };
    expect(clientsParityProblems(drifted)).toEqual([
      "qa runs check:svelte, but the clients job neither runs it nor declares it omitted",
    ]);
  });

  test("a job-level if on the clients job leaves every family gate uncovered", async () => {
    const inputs = await realInputs();
    const drifted = {
      ...inputs,
      workflow: inputs.workflow.replace(
        "  clients:\n    name:",
        "  clients:\n    if: false\n    name:",
      ),
    };
    const problems = clientsParityProblems(drifted);
    expect(problems).toContain(
      "qa runs check:ts, but the clients job neither runs it nor declares it omitted",
    );
    expect(problems).toContain(
      "qa runs check:bindings, but the clients job neither runs it nor declares it omitted",
    );
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

  test("a guarded effigy qa step in release.yml does not cover the omissions", async () => {
    const inputs = await realInputs();
    const drifted = {
      ...inputs,
      releaseWorkflow: inputs.releaseWorkflow.replace(
        "        run: effigy qa",
        "        if: false\n        run: effigy qa",
      ),
    };
    expect(clientsParityProblems(drifted)).toContain(
      "release.yml no longer runs `effigy qa`, so the omitted gates have no covering run",
    );
  });

  test("a job-level if on release.yml's job does not cover the omissions", async () => {
    const inputs = await realInputs();
    const drifted = {
      ...inputs,
      releaseWorkflow: inputs.releaseWorkflow.replace(
        "\njobs:\n  release:\n",
        "\njobs:\n  release:\n    if: false\n",
      ),
    };
    expect(clientsParityProblems(drifted)).toContain(
      "release.yml no longer runs `effigy qa`, so the omitted gates have no covering run",
    );
  });

  test("an effigy qa line that is only echoed in release.yml does not cover the omissions", async () => {
    const inputs = await realInputs();
    const drifted = {
      ...inputs,
      releaseWorkflow: inputs.releaseWorkflow.replace(
        "        run: effigy qa",
        "        run: echo effigy qa",
      ),
    };
    expect(clientsParityProblems(drifted)).toContain(
      "release.yml no longer runs `effigy qa`, so the omitted gates have no covering run",
    );
  });

  test("release.yml that stops running effigy qa leaves the omissions uncovered", async () => {
    const inputs = await realInputs();
    const problems = clientsParityProblems({ ...inputs, releaseWorkflow: "name: release\n" });
    expect(problems).toEqual([
      "release.yml no longer runs `effigy qa`, so the omitted gates have no covering run",
    ]);
  });
});
