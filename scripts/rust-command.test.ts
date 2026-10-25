import { afterEach, describe, expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, readFile, rm, stat, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  privateCargoCommand,
  rustCommand,
  rustExec,
  taskScratch,
} from "./rust-command.ts";

const helper = fileURLToPath(new URL("./rust-command.sh", import.meta.url));
const privateHelper = fileURLToPath(new URL("./private-cargo.sh", import.meta.url));
const temporaryDirectories: string[] = [];

afterEach(async () => {
  await Promise.all(temporaryDirectories.splice(0).map((path) => rm(path, { recursive: true, force: true })));
});

/**
 * Fake tools that record the tool name, every argument and the environment the
 * switch point handed them. No real Cargo or Oxide runs in these tests.
 */
async function sandbox(tools: string[]) {
  const root = await mkdtemp(join(tmpdir(), "longhorn-rust-command-"));
  temporaryDirectories.push(root);
  const bin = join(root, "bin");
  await mkdir(bin);
  const capture = join(root, "invocation");

  for (const tool of tools) {
    const path = join(bin, tool);
    await writeFile(
      path,
      `#!/bin/sh
{
  printf 'tool=%s\\n' "${tool}"
  for argument in "$@"; do printf 'arg=%s\\n' "$argument"; done
  printf 'env=RUSTUP_TOOLCHAIN=%s\\n' "\${RUSTUP_TOOLCHAIN-<unset>}"
  printf 'env=CARGO_TARGET_DIR=%s\\n' "\${CARGO_TARGET_DIR-<unset>}"
  printf 'env=CARGO_BUILD_BUILD_DIR=%s\\n' "\${CARGO_BUILD_BUILD_DIR-<unset>}"
} > "$RUST_COMMAND_CAPTURE"
exit "\${FAKE_TOOL_EXIT:-0}"
`,
    );
    await chmod(path, 0o755);
  }

  return { bin, capture };
}

type Invocation = {
  exitCode: number;
  stdout: string;
  stderr: string;
  run: string;
};

async function invoke(
  bin: string,
  capture: string,
  arguments_: readonly string[],
  environment: Record<string, string> = {},
): Promise<Invocation> {
  const proc = Bun.spawn(["/bin/bash", helper, ...arguments_], {
    env: {
      PATH: `${bin}:/bin`,
      HOME: process.env.HOME ?? "/tmp",
      RUST_COMMAND_CAPTURE: capture,
      GITHUB_ACTIONS: "",
      EFFIGY_OXIDE_ESCAPE: "",
      ...environment,
    },
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    proc.exited,
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  const run = await readFile(capture, "utf8").catch(() => "");
  return { exitCode, stdout, stderr, run };
}

const CLIPPY = ["clippy", "--workspace", "--", "-D", "warnings"];
const EXEC_TOOL = ["cargo", "run", "-p", "longhorn-bindings", "--", "bridge", "check"];

describe("Rust command routing", () => {
  test("uses Oxide locally and preserves arguments", async () => {
    const { bin, capture } = await sandbox(["oxide", "cargo"]);
    const result = await invoke(bin, capture, CLIPPY);

    expect(result.exitCode).toBe(0);
    expect(result.stderr).toContain("rust runner: oxide clippy --workspace -- -D warnings");
    expect(result.run).toContain("tool=oxide\narg=clippy\narg=--workspace\narg=--\narg=-D\narg=warnings\n");
  });

  test("uses Cargo on GitHub Actions even without Oxide", async () => {
    const { bin, capture } = await sandbox(["cargo"]);
    const result = await invoke(bin, capture, CLIPPY, { GITHUB_ACTIONS: "true" });

    expect(result.exitCode).toBe(0);
    expect(result.stderr).toContain("rust runner: cargo clippy --workspace -- -D warnings");
    expect(result.run).toContain("tool=cargo\narg=clippy\narg=--workspace\narg=--\narg=-D\narg=warnings\n");
  });

  test("the explicit escape runs Cargo locally without Oxide", async () => {
    const { bin, capture } = await sandbox(["cargo"]);
    const result = await invoke(bin, capture, CLIPPY, { EFFIGY_OXIDE_ESCAPE: "cargo" });

    expect(result.exitCode).toBe(0);
    expect(result.stderr).toContain("rust runner: cargo clippy --workspace -- -D warnings");
    expect(result.run).toContain("tool=cargo\narg=clippy\n");
  });

  test("refuses an unsupported escape value instead of ignoring it", async () => {
    const { bin, capture } = await sandbox(["oxide", "cargo"]);
    for (const value of ["oxide", "true", "Cargo", "1"]) {
      const result = await invoke(bin, capture, CLIPPY, { EFFIGY_OXIDE_ESCAPE: value });
      expect(result.exitCode).toBe(2);
      expect(result.stderr).toContain(`EFFIGY_OXIDE_ESCAPE=${value} is unsupported`);
      expect(result.run).toBe("");
    }
  });

  test("fails locally without Oxide and names the escape", async () => {
    const { bin, capture } = await sandbox([]);
    const result = await invoke(bin, capture, CLIPPY);

    expect(result.exitCode).toBe(127);
    expect(result.stderr).toContain("oxide is required");
    expect(result.stderr).toContain("EFFIGY_OXIDE_ESCAPE=cargo");
    expect(result.stderr).not.toContain("lint:rust:cargo");
  });

  test("rejects an empty Cargo command", async () => {
    const { bin, capture } = await sandbox(["oxide", "cargo"]);
    const result = await invoke(bin, capture, []);

    expect(result.exitCode).toBe(2);
    expect(result.stderr).toContain("usage: rust-command.sh");
  });

  test("propagates the runner's exit code", async () => {
    const { bin, capture } = await sandbox(["oxide", "cargo"]);
    const result = await invoke(bin, capture, CLIPPY, { FAKE_TOOL_EXIT: "7" });

    expect(result.exitCode).toBe(7);
  });
});

describe("Rust exec routing", () => {
  test("leases one workspace through Oxide exec locally and keeps the tool argv", async () => {
    const { bin, capture } = await sandbox(["oxide", "cargo"]);
    const result = await invoke(bin, capture, [
      "exec",
      "--manifest-path",
      "/scratch/staged/Cargo.toml",
      "--profile",
      "release",
      "--",
      ...EXEC_TOOL,
    ]);

    expect(result.exitCode).toBe(0);
    expect(result.stderr).toContain(
      "rust runner: oxide exec --manifest-path /scratch/staged/Cargo.toml --profile release -- cargo run -p longhorn-bindings -- bridge check",
    );
    expect(result.run).toContain(
      [
        "tool=oxide",
        "arg=exec",
        "arg=--manifest-path",
        "arg=/scratch/staged/Cargo.toml",
        "arg=--profile",
        "arg=release",
        "arg=--",
        ...EXEC_TOOL.map((argument) => `arg=${argument}`),
        "",
      ].join("\n"),
    );
  });

  test("omits Oxide's options it was not given", async () => {
    const { bin, capture } = await sandbox(["oxide", "cargo"]);
    const result = await invoke(bin, capture, ["exec", "--", ...EXEC_TOOL]);

    expect(result.exitCode).toBe(0);
    expect(result.run.startsWith("tool=oxide\narg=exec\narg=--\narg=cargo\n")).toBe(true);
    expect(result.run).not.toContain("--manifest-path");
    expect(result.run).not.toContain("--profile");
  });

  test("pins the toolchain before the lease and hands it to the tool", async () => {
    const { bin, capture } = await sandbox(["oxide", "cargo"]);
    const result = await invoke(bin, capture, [
      "exec",
      "--manifest-path",
      "/scratch/staged/Cargo.toml",
      "--",
      ...EXEC_TOOL,
    ], { RUSTUP_TOOLCHAIN: "1.95.0" });

    expect(result.exitCode).toBe(0);
    expect(result.run).toContain("env=RUSTUP_TOOLCHAIN=1.95.0");
    expect(result.run).toContain("arg=--manifest-path\narg=/scratch/staged/Cargo.toml\narg=--\narg=cargo\n");
  });

  test("runs the tool directly on GitHub Actions", async () => {
    const { bin, capture } = await sandbox(["oxide", "cargo"]);
    const result = await invoke(bin, capture, [
      "exec",
      "--manifest-path",
      "/scratch/staged/Cargo.toml",
      "--profile",
      "release",
      "--",
      ...EXEC_TOOL,
    ], { GITHUB_ACTIONS: "true" });

    expect(result.exitCode).toBe(0);
    expect(result.stderr).toContain("rust runner: cargo exec");
    expect(result.run).toContain([`tool=cargo`, ...EXEC_TOOL.slice(1).map((argument) => `arg=${argument}`), ""].join("\n"));
  });

  test("runs the tool directly under the explicit escape", async () => {
    const { bin, capture } = await sandbox(["cargo"]);
    const result = await invoke(bin, capture, [
      "exec",
      "--manifest-path",
      "/scratch/staged/Cargo.toml",
      "--",
      ...EXEC_TOOL,
    ], { EFFIGY_OXIDE_ESCAPE: "cargo" });

    expect(result.exitCode).toBe(0);
    expect(result.run).toContain([`tool=cargo`, ...EXEC_TOOL.slice(1).map((argument) => `arg=${argument}`), ""].join("\n"));
  });

  test("refuses an unsupported profile and a missing tool", async () => {
    const { bin, capture } = await sandbox(["oxide", "cargo"]);
    const badProfile = await invoke(bin, capture, ["exec", "--profile", "fast", "--", ...EXEC_TOOL]);
    expect(badProfile.exitCode).toBe(2);
    expect(badProfile.stderr).toContain("--profile fast is not quick, debug or release");

    const noTool = await invoke(bin, capture, ["exec", "--manifest-path", "/x/Cargo.toml", "--"]);
    expect(noTool.exitCode).toBe(2);
    expect(noTool.stderr).toContain("usage: rust-command.sh");

    const stray = await invoke(bin, capture, ["exec", "cargo", "run"]);
    expect(stray.exitCode).toBe(2);
    expect(stray.stderr).toContain("unexpected argument cargo before --");
    expect(await readFile(capture, "utf8").catch(() => "")).toBe("");
  });
});

describe("Private Cargo outputs", () => {
  async function invokePrivate(
    bin: string,
    capture: string,
    arguments_: readonly string[],
  ): Promise<Invocation> {
    const proc = Bun.spawn(["/bin/bash", privateHelper, ...arguments_], {
      env: {
        PATH: `${bin}:/bin`,
        HOME: process.env.HOME ?? "/tmp",
        RUST_COMMAND_CAPTURE: capture,
        CARGO_BUILD_BUILD_DIR: "/pool/slot/build",
      },
      stdout: "pipe",
      stderr: "pipe",
    });
    const [exitCode, stdout, stderr] = await Promise.all([
      proc.exited,
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
    ]);
    const run = await readFile(capture, "utf8").catch(() => "");
    return { exitCode, stdout, stderr, run };
  }

  test("moves final outputs and leaves the pooled build directory alone", async () => {
    const { bin, capture } = await sandbox(["cargo"]);
    const result = await invokePrivate(bin, capture, [
      "--target",
      "/scratch/final",
      "--",
      "run",
      "-p",
      "longhorn-bindings",
      "--offline",
    ]);

    expect(result.exitCode).toBe(0);
    expect(result.run).toContain("tool=cargo\narg=run\narg=-p\narg=longhorn-bindings\narg=--offline\n");
    expect(result.run).toContain("env=CARGO_TARGET_DIR=/scratch/final");
    expect(result.run).toContain("env=CARGO_BUILD_BUILD_DIR=/pool/slot/build");
  });

  test("pins the build directory too for the release-absence scans", async () => {
    const { bin, capture } = await sandbox(["cargo"]);
    const result = await invokePrivate(bin, capture, [
      "--target",
      "/scratch/scan",
      "--build-dir",
      "--",
      "build",
      "-p",
      "longhorn-agent-control",
      "--release",
    ]);

    expect(result.exitCode).toBe(0);
    expect(result.run).toContain("env=CARGO_TARGET_DIR=/scratch/scan");
    expect(result.run).toContain("env=CARGO_BUILD_BUILD_DIR=/scratch/scan");
  });

  test("accepts the command with or without its leading cargo", async () => {
    const { bin, capture } = await sandbox(["cargo"]);
    const withTool = await invokePrivate(bin, capture, [
      "--target",
      "/scratch/final",
      "--",
      "cargo",
      "check",
      "--locked",
    ]);
    expect(withTool.exitCode).toBe(0);
    expect(withTool.run).toContain("tool=cargo\narg=check\narg=--locked\n");

    const withoutTool = await invokePrivate(bin, capture, [
      "--target",
      "/scratch/final",
      "--",
      "check",
      "--locked",
    ]);
    expect(withoutTool.exitCode).toBe(0);
    expect(withoutTool.run).toContain("tool=cargo\narg=check\narg=--locked\n");
  });

  test("refuses a call without a target or a command", async () => {
    const { bin, capture } = await sandbox(["cargo"]);
    const noTarget = await invokePrivate(bin, capture, ["--", "run"]);
    expect(noTarget.exitCode).toBe(2);
    expect(noTarget.stderr).toContain("usage: private-cargo.sh");

    const noCommand = await invokePrivate(bin, capture, ["--target", "/scratch/final", "--"]);
    expect(noCommand.exitCode).toBe(2);

    const stray = await invokePrivate(bin, capture, ["/scratch/final", "run"]);
    expect(stray.exitCode).toBe(2);
    expect(stray.stderr).toContain("unexpected argument /scratch/final before --");
    expect(await readFile(capture, "utf8").catch(() => "")).toBe("");
  });
});

describe("Switch point argv", () => {
  test("rustCommand feeds the Cargo form", () => {
    expect(rustCommand(["tree", "-p", "longhorn-core"])).toEqual([
      "bash",
      helper,
      "tree",
      "-p",
      "longhorn-core",
    ]);
    expect(rustCommand(["package", "--list"], "1.95.0")).toEqual([
      "env",
      "RUSTUP_TOOLCHAIN=1.95.0",
      "bash",
      helper,
      "package",
      "--list",
    ]);
  });

  test("rustExec feeds the exec form", () => {
    expect(
      rustExec(["cargo", "run", "-p", "proof"], {
        manifestPath: "/scratch/staged/Cargo.toml",
        profile: "release",
        toolchain: "1.95.0",
      }),
    ).toEqual([
      "env",
      "RUSTUP_TOOLCHAIN=1.95.0",
      "bash",
      helper,
      "exec",
      "--manifest-path",
      "/scratch/staged/Cargo.toml",
      "--profile",
      "release",
      "--",
      "cargo",
      "run",
      "-p",
      "proof",
    ]);

    expect(rustExec(["cargo", "check"], { manifestPath: "/x/Cargo.toml" })).not.toContain(
      "--profile",
    );
  });

  test("privateCargoCommand feeds the private output helper", () => {
    expect(privateCargoCommand("/scratch/final", ["cargo", "run"])).toEqual([
      "bash",
      privateHelper,
      "--target",
      "/scratch/final",
      "--",
      "cargo",
      "run",
    ]);
    expect(
      privateCargoCommand("/scratch/scan", ["cargo", "build"], { buildDir: true }),
    ).toEqual([
      "bash",
      privateHelper,
      "--target",
      "/scratch/scan",
      "--build-dir",
      "--",
      "cargo",
      "build",
    ]);
  });
});

describe("Task scratch", () => {
  test("allocates on the configured scratch root and removes only its own directory", async () => {
    const base = await mkdtemp(join(tmpdir(), "longhorn-scratch-root-"));
    temporaryDirectories.push(base);
    const previous = process.env.OXIDE_TASK_SCRATCH_ROOT;
    process.env.OXIDE_TASK_SCRATCH_ROOT = base;
    try {
      const scratch = await taskScratch("longhorn-proof");
      expect(scratch.root.startsWith(join(base, "longhorn-proof-"))).toBe(true);
      expect((await stat(scratch.root)).isDirectory()).toBe(true);
      await scratch.dispose();
      await expect(stat(scratch.root)).rejects.toThrow();
      expect((await stat(base)).isDirectory()).toBe(true);
    } finally {
      if (previous === undefined) delete process.env.OXIDE_TASK_SCRATCH_ROOT;
      else process.env.OXIDE_TASK_SCRATCH_ROOT = previous;
    }
  });

  test("uses runner-owned temporary storage on GitHub Actions", async () => {
    const previous = process.env.GITHUB_ACTIONS;
    const previousRoot = process.env.OXIDE_TASK_SCRATCH_ROOT;
    process.env.GITHUB_ACTIONS = "true";
    process.env.OXIDE_TASK_SCRATCH_ROOT = "/nonexistent/scratch";
    try {
      const scratch = await taskScratch("longhorn-ci-proof");
      expect(scratch.root.startsWith(join(tmpdir(), "longhorn-ci-proof-"))).toBe(true);
      await scratch.dispose();
    } finally {
      if (previous === undefined) delete process.env.GITHUB_ACTIONS;
      else process.env.GITHUB_ACTIONS = previous;
      if (previousRoot === undefined) delete process.env.OXIDE_TASK_SCRATCH_ROOT;
      else process.env.OXIDE_TASK_SCRATCH_ROOT = previousRoot;
    }
  });

  test("fails clearly when the external scratch root is unavailable", async () => {
    const previous = process.env.OXIDE_TASK_SCRATCH_ROOT;
    const previousActions = process.env.GITHUB_ACTIONS;
    const sandboxRoot = await mkdtemp(join(tmpdir(), "longhorn-scratch-file-"));
    temporaryDirectories.push(sandboxRoot);
    const notADirectory = join(sandboxRoot, "file");
    await writeFile(notADirectory, "");
    delete process.env.GITHUB_ACTIONS;
    process.env.OXIDE_TASK_SCRATCH_ROOT = join(notADirectory, "scratch");
    try {
      await expect(taskScratch("longhorn-missing")).rejects.toThrow(
        /cannot reach build scratch .*OXIDE_TASK_SCRATCH_ROOT/,
      );
    } finally {
      if (previous === undefined) delete process.env.OXIDE_TASK_SCRATCH_ROOT;
      else process.env.OXIDE_TASK_SCRATCH_ROOT = previous;
      if (previousActions === undefined) delete process.env.GITHUB_ACTIONS;
      else process.env.GITHUB_ACTIONS = previousActions;
    }
  });
});
