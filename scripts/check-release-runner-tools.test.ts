import { describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { checkCiRunnerTools } from "./check-ci-runner-tools.ts";
import {
  cargoInstallCrates,
  checkReleaseRunnerTools,
  formatReleaseRunnerToolsFailure,
  parseWorkflowInstalls,
  toolsNeededByCommand,
  workflowRunScripts,
} from "./check-release-runner-tools.ts";

const gates = `
[release.gates]
private-candidate = "bun scripts/verify-private-candidate-docs-card127.ts"
advisories = "cargo deny check advisories"
rustdoc = "effigy docs:rust"
prototypes = "effigy check:prototypes"
workspace = "effigy qa"
floor = "effigy release:floor"
source = "effigy release:source-consumer"
`;

const setup = `
      - uses: dtolnay/rust-toolchain@abc
      - uses: oven-sh/setup-bun@abc
      - uses: inflatable-cookie/setup-effigy@abc
`;

const ciBunSetup = `      - uses: oven-sh/setup-bun@0c5077e51419868618aeaa5fe8019c62421857d6 # v2
        with:
          bun-version: '1.3.14' # pinned: the version local qa runs
`;

const ciWorkflow = (rustBun = ciBunSetup, floorBun = ciBunSetup) => `
name: CI
jobs:
  clients:
    steps:
${ciBunSetup}      - run: effigy check:ts
  rust:
    steps:
${rustBun}      - run: effigy test:rust
  rust-floor:
    steps:
${floorBun}      - run: effigy release:floor
`;

const workflowWithDeny = `
name: Release
jobs:
  release:
    steps:
${setup}
      # The advisories gate runs cargo deny. A comment is not an install.
      - name: Install cargo-deny
        run: cargo install cargo-deny --locked --version 0.19.4
      - name: Release gates
        run: effigy release:gates
`;

const workflowWithoutDeny = `
name: Release
jobs:
  release:
    steps:
${setup}
      # The advisories gate runs cargo deny. A comment is not an install.
      - name: Install cargo-deny
        run: echo "forgot the install"
      - name: Release gates
        run: effigy release:gates
`;

const workflowDenyAfterGates = `
name: Release
jobs:
  release:
    steps:
${setup}
      - name: Release gates
        run: effigy release:gates
      - name: Install cargo-deny
        run: cargo install cargo-deny --locked --version 0.19.4
`;

const workflowDenyConditional = `
name: Release
jobs:
  release:
    steps:
${setup}
      - name: Install cargo-deny
        if: \${{ false }}
        run: cargo install cargo-deny --locked --version 0.19.4
      - name: Release gates
        run: effigy release:gates
`;

const workflowDenySameScriptAfter = `
name: Release
jobs:
  release:
    steps:
${setup}
      - name: Gates then install
        run: |
          effigy release:gates
          cargo install cargo-deny --locked --version 0.19.4
`;

const workflowDenyOtherJob = `
name: Release
jobs:
  install:
    steps:
${setup}
      - name: Install cargo-deny
        run: cargo install cargo-deny --locked --version 0.19.4
  release:
    steps:
${setup}
      - name: Release gates
        run: effigy release:gates
`;

describe("release-gate runner tools", () => {
  test("cargo deny needs cargo-deny; stock cargo subcommands do not", () => {
    expect(toolsNeededByCommand("cargo deny check advisories").map((item) => item.tool)).toEqual([
      "cargo",
      "cargo-deny",
    ]);
    expect(toolsNeededByCommand("cargo test --workspace").map((item) => item.tool)).toEqual(["cargo"]);
    expect(toolsNeededByCommand("effigy docs:rust").map((item) => item.tool)).toEqual(["effigy"]);
    expect(toolsNeededByCommand("bun scripts/verify-private-candidate-docs-card127.ts").map((item) => item.tool)).toEqual([
      "bun",
    ]);
  });

  test("sh -c with an inner cargo plugin is mapped; a bare sh -c is not", () => {
    expect(toolsNeededByCommand("sh -c 'cargo deny check advisories'").map((item) => item.tool)).toEqual([
      "cargo",
      "cargo-deny",
    ]);
    expect(() => toolsNeededByCommand("sh -c 'echo hi'")).toThrow(/without executing/);
  });

  test("cargo install is taken from run scripts, not comments or step names", () => {
    const scripts = workflowRunScripts(workflowWithDeny);
    expect(scripts).toContain("cargo install cargo-deny --locked --version 0.19.4");
    expect(cargoInstallCrates("cargo install cargo-deny --locked --version 0.19.4")).toEqual(["cargo-deny"]);
    expect(parseWorkflowInstalls(workflowWithDeny).tools.has("cargo-deny")).toBe(true);
    expect(parseWorkflowInstalls(workflowWithoutDeny).tools.has("cargo-deny")).toBe(false);
    expect(cargoInstallCrates("echo cargo install cargo-deny")).toEqual([]);
  });

  test("missing cargo-deny install fails and names the tool", async () => {
    const missing = await writeRepo(gates, workflowWithoutDeny);
    const result = checkReleaseRunnerTools(missing);
    const denied = result.failures.filter((item) => item.tool === "cargo-deny");
    expect(denied.length).toBe(1);
    expect(denied[0]?.gate).toBe("advisories");
    expect(formatReleaseRunnerToolsFailure(result)).toContain("cargo-deny");
    expect(formatReleaseRunnerToolsFailure(result)).toContain("advisories");
  });

  test("cargo install of cargo-deny satisfies the advisories gate", async () => {
    const present = await writeRepo(gates, workflowWithDeny);
    const result = checkReleaseRunnerTools(present);
    expect(result.failures).toEqual([]);
    expect(
      result.requirements.some((item) => item.tool === "cargo-deny" && item.via === "cargo install cargo-deny"),
    ).toBe(true);
  });

  test("cargo-deny install after release:gates does not satisfy the advisories gate", async () => {
    const after = await writeRepo(gates, workflowDenyAfterGates);
    const result = checkReleaseRunnerTools(after);
    expect(result.failures.some((item) => item.tool === "cargo-deny")).toBe(true);
    expect(parseWorkflowInstalls(workflowDenyAfterGates).tools.has("cargo-deny")).toBe(false);
    expect(parseWorkflowInstalls(workflowDenyConditional).tools.has("cargo-deny")).toBe(false);
    expect(parseWorkflowInstalls(workflowDenySameScriptAfter).tools.has("cargo-deny")).toBe(false);
  });

  test("cargo-deny install in another job does not satisfy the advisories gate", async () => {
    const other = await writeRepo(gates, workflowDenyOtherJob);
    const result = checkReleaseRunnerTools(other);
    const denied = result.failures.filter((item) => item.tool === "cargo-deny");
    expect(denied.length).toBe(1);
    expect(denied[0]?.gate).toBe("advisories");
    expect(parseWorkflowInstalls(workflowDenyOtherJob).tools.has("cargo-deny")).toBe(false);
    expect(formatReleaseRunnerToolsFailure(result)).toContain("cargo-deny");
  });

  test("live catalog matches: cargo-deny is installed, qa passes the check", () => {
    const live = checkReleaseRunnerTools();
    expect(live.failures).toEqual([]);
    const deny = live.requirements.find((item) => item.tool === "cargo-deny");
    expect(deny?.gate).toBe("advisories");
    expect(deny?.provided).toBe(true);
    expect(deny?.via).toBe("cargo install cargo-deny");
  });
});

describe("CI Rust runner tools", () => {
  test("Rust and floor install the same pinned Bun before Effigy", () => {
    expect(checkCiRunnerTools(ciWorkflow())).toEqual([]);
  });

  test("missing Bun in the Rust job fails", () => {
    expect(checkCiRunnerTools(ciWorkflow("", ciBunSetup))).toContain(
      "rust must install Bun before its first Effigy step",
    );
  });

  test("Bun installed after Effigy fails", () => {
    const afterEffigy = `      - run: effigy release:floor\n${ciBunSetup}`;
    expect(checkCiRunnerTools(ciWorkflow(ciBunSetup, afterEffigy))).toContain(
      "rust-floor must install Bun before its first Effigy step",
    );
  });

  test("a different Bun pin fails", () => {
    const differentVersion = ciBunSetup.replace("1.3.14", "1.3.13");
    expect(checkCiRunnerTools(ciWorkflow(ciBunSetup, differentVersion))).toContain(
      "rust-floor must use the same pinned Bun action and version as clients",
    );
  });

  test("the live CI workflow keeps both Rust jobs aligned with clients", async () => {
    const workflow = await readFile(join(import.meta.dir, "../.github/workflows/ci.yml"), "utf8");
    expect(checkCiRunnerTools(workflow)).toEqual([]);
  });
});

async function writeRepo(releaseToml: string, workflow: string): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "longhorn-release-runner-tools-"));
  await mkdir(join(root, "config"), { recursive: true });
  await mkdir(join(root, ".github/workflows"), { recursive: true });
  await writeFile(join(root, "config/release.toml"), releaseToml);
  await writeFile(join(root, ".github/workflows/release.yml"), workflow);
  return root;
}
