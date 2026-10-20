import { afterEach, describe, expect, test } from "bun:test";
import { chmod, mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const helper = fileURLToPath(new URL("./rust-command.sh", import.meta.url));
const temporaryDirectories: string[] = [];

afterEach(async () => {
  await Promise.all(temporaryDirectories.splice(0).map((path) => rm(path, { recursive: true, force: true })));
});

async function sandbox(tools: string[]) {
  const root = await mkdtemp(join(tmpdir(), "longhorn-rust-command-"));
  temporaryDirectories.push(root);
  const bin = join(root, "bin");
  await mkdir(bin);
  const capture = join(root, "invocation");

  for (const tool of tools) {
    const path = join(bin, tool);
    await writeFile(path, '#!/bin/sh\nprintf "%s\\n" "$0" "$@" > "$RUST_COMMAND_CAPTURE"\n');
    await chmod(path, 0o755);
  }

  return { bin, capture };
}

async function invoke(bin: string, capture: string, githubActions?: string) {
  const proc = Bun.spawn(["/bin/bash", helper, "clippy", "--workspace", "--", "-D", "warnings"], {
    env: {
      ...process.env,
      PATH: `${bin}:/bin`,
      RUST_COMMAND_CAPTURE: capture,
      ...(githubActions === undefined ? { GITHUB_ACTIONS: "" } : { GITHUB_ACTIONS: githubActions }),
    },
    stdout: "pipe",
    stderr: "pipe",
  });
  const [exitCode, stdout, stderr] = await Promise.all([
    proc.exited,
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  return { exitCode, stdout, stderr };
}

describe("Rust command routing", () => {
  test("uses Oxide locally and preserves arguments", async () => {
    const { bin, capture } = await sandbox(["oxide", "cargo"]);
    const result = await invoke(bin, capture);

    expect(result.exitCode).toBe(0);
    expect(result.stderr).toContain("rust runner: oxide clippy --workspace -- -D warnings");
    expect(await readFile(capture, "utf8")).toContain("/oxide\nclippy\n--workspace\n--\n-D\nwarnings\n");
  });

  test("uses Cargo on GitHub Actions even without Oxide", async () => {
    const { bin, capture } = await sandbox(["cargo"]);
    const result = await invoke(bin, capture, "true");

    expect(result.exitCode).toBe(0);
    expect(result.stderr).toContain("rust runner: cargo clippy --workspace -- -D warnings");
    expect(await readFile(capture, "utf8")).toContain("/cargo\nclippy\n--workspace\n--\n-D\nwarnings\n");
  });

  test("fails locally without Oxide and names the Cargo escapes", async () => {
    const { bin, capture } = await sandbox([]);
    const result = await invoke(bin, capture);

    expect(result.exitCode).toBe(127);
    expect(result.stderr).toContain("oxide is required");
    expect(result.stderr).toContain("effigy lint:rust:cargo");
    expect(result.stderr).toContain("effigy test:rust:cargo");
    expect(result.stderr).toContain("effigy test:pty:cargo");
  });
});
