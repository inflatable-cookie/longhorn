import { describe, expect, test } from "bun:test";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

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

const workflowWithDeny = `
name: Release
jobs:
  release:
    steps:
      - uses: dtolnay/rust-toolchain@abc
      - uses: oven-sh/setup-bun@abc
      - uses: inflatable-cookie/setup-effigy@abc
      # The advisories gate runs cargo deny. A comment is not an install.
      - name: Install cargo-deny
        run: cargo install cargo-deny --locked --version 0.19.4
`;

const workflowWithoutDeny = `
name: Release
jobs:
  release:
    steps:
      - uses: dtolnay/rust-toolchain@abc
      - uses: oven-sh/setup-bun@abc
      - uses: inflatable-cookie/setup-effigy@abc
      # The advisories gate runs cargo deny. A comment is not an install.
      - name: Install cargo-deny
        run: echo "forgot the install"
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
    expect(scripts).toEqual(["cargo install cargo-deny --locked --version 0.19.4"]);
    expect(cargoInstallCrates(scripts[0]!)).toEqual(["cargo-deny"]);
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

  test("live catalog matches: cargo-deny is installed, qa passes the check", () => {
    const live = checkReleaseRunnerTools();
    expect(live.failures).toEqual([]);
    const deny = live.requirements.find((item) => item.tool === "cargo-deny");
    expect(deny?.gate).toBe("advisories");
    expect(deny?.provided).toBe(true);
    expect(deny?.via).toBe("cargo install cargo-deny");
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
