import { afterEach, describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  cargoInstallCrates,
  checkReleaseRunnerTools,
  formatReleaseRunnerToolsFailure,
  checkCiRunnerTools,
  findForbiddenTools,
  releaseWorkflowInstalls,
  toolsNeededByCommand,
} from "./check-runner-tools.ts";

const gates = `
[release.gates]
private-candidate = "bun scripts/verify-private-candidate-docs.ts"
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
    expect(toolsNeededByCommand("cargo deny check advisories")).toEqual([
      "cargo",
      "cargo-deny",
    ]);
    expect(toolsNeededByCommand("cargo test --workspace")).toEqual(["cargo"]);
    expect(toolsNeededByCommand("effigy docs:rust")).toEqual(["effigy"]);
    expect(toolsNeededByCommand("bun scripts/verify-private-candidate-docs.ts")).toEqual([
      "bun",
    ]);
  });

  test("sh -c with an inner cargo plugin is mapped; a bare sh -c is not", () => {
    expect(toolsNeededByCommand("sh -c 'cargo deny check advisories'")).toEqual([
      "cargo",
      "cargo-deny",
    ]);
    expect(() => toolsNeededByCommand("sh -c 'echo hi'")).toThrow(/without executing/);
  });

  test("the rust-command.sh switch point keeps the runner Cargo identity", () => {
    // Locally that gate runs `oxide`; a runner still needs `cargo` and the
    // plugin, so the launcher must not hide the requirement.
    expect(
      toolsNeededByCommand("bash scripts/rust-command.sh deny check advisories"),
    ).toEqual(["cargo", "cargo-deny"]);
    expect(
      toolsNeededByCommand("bash scripts/rust-command.sh test --workspace"),
    ).toEqual(["cargo"]);
  });

  test("cargo install is taken from run scripts, not comments or step names", () => {
    expect(cargoInstallCrates("cargo install cargo-deny --locked --version 0.19.4")).toEqual(["cargo-deny"]);
    expect(releaseWorkflowInstalls(workflowWithDeny).has("cargo-deny")).toBe(true);
    expect(releaseWorkflowInstalls(workflowWithoutDeny).has("cargo-deny")).toBe(false);
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
    expect(releaseWorkflowInstalls(workflowDenyAfterGates).has("cargo-deny")).toBe(false);
    expect(releaseWorkflowInstalls(workflowDenyConditional).has("cargo-deny")).toBe(false);
    expect(releaseWorkflowInstalls(workflowDenySameScriptAfter).has("cargo-deny")).toBe(false);
  });

  test("cargo-deny install in another job does not satisfy the advisories gate", async () => {
    const other = await writeRepo(gates, workflowDenyOtherJob);
    const result = checkReleaseRunnerTools(other);
    const denied = result.failures.filter((item) => item.tool === "cargo-deny");
    expect(denied.length).toBe(1);
    expect(denied[0]?.gate).toBe("advisories");
    expect(releaseWorkflowInstalls(workflowDenyOtherJob).has("cargo-deny")).toBe(false);
    expect(formatReleaseRunnerToolsFailure(result)).toContain("cargo-deny");
  });

  test("live catalog matches: cargo-deny is installed, qa passes the check", () => {
    const live = checkReleaseRunnerTools();
    expect(live.failures).toEqual([]);
    const deny = live.requirements.find((item) => item.tool === "cargo-deny");
    expect(deny?.gate).toBe("advisories");
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

describe("workflow YAML and runner source", () => {
  test("handles folded commands, quoted actions and YAML comments", () => {
    const workflow = workflowWithDeny
      .replace("uses: oven-sh/setup-bun@abc", "uses: 'oven-sh/setup-bun@abc' # setup")
      .replace(
        "run: cargo install cargo-deny --locked --version 0.19.4",
        "run: >-\n          cargo install cargo-deny\n          --locked --version 0.19.4",
      )
      .replace("run: effigy release:gates", "run: 'effigy release:gates' # gate");
    const installs = releaseWorkflowInstalls(workflow);
    expect(installs.get("bun")).toBe("oven-sh/setup-bun");
    expect(installs.get("cargo-deny")).toBe("cargo install cargo-deny");
  });

  test("every release-gates job needs its own install", () => {
    const secondJob = workflowWithoutDeny.slice(workflowWithoutDeny.indexOf("  release:"));
    const workflow = workflowWithDeny + secondJob.replace("  release:", "  other:");
    expect(releaseWorkflowInstalls(workflow).has("cargo-deny")).toBe(false);
  });

  test("a conditional Bun action cannot satisfy the Rust job", () => {
    const conditionalBun = ciBunSetup.replace("        with:", "        if: false\n        with:");
    expect(checkCiRunnerTools(ciWorkflow(conditionalBun))).toContain(
      "rust must install Bun before its first Effigy step",
    );
  });

  test("reports forbidden commands with their file and line", () => {
    const shortName = "r" + "g";
    const longName = "rip" + "grep";
    expect(findForbiddenTools("scripts/example.sh", [
      `# ${shortName} is only mentioned`,
      `find . | ${shortName} needle`,
      `${longName} pattern # command`,
    ].join("\n"))).toEqual([
      { file: "scripts/example.sh", line: 2, tool: shortName, text: `find . | ${shortName} needle` },
      { file: "scripts/example.sh", line: 3, tool: longName, text: `${longName} pattern # command` },
    ]);
    expect(findForbiddenTools("effigy.toml", `# Prefer grep over ${shortName} commands`)).toEqual([]);
  });
});

const fixtures: string[] = [];
afterEach(async () => {
  await Promise.all(fixtures.splice(0).map((root) => rm(root, { recursive: true, force: true })));
});

async function writeRepo(releaseToml: string, workflow: string): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "longhorn-release-runner-tools-"));
  fixtures.push(root);
  await mkdir(join(root, "config"), { recursive: true });
  await mkdir(join(root, ".github/workflows"), { recursive: true });
  await writeFile(join(root, "config/release.toml"), releaseToml);
  await writeFile(join(root, ".github/workflows/release.yml"), workflow);
  return root;
}
