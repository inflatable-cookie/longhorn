// Reproduces g02.036 L1 in a disposable provider-free source consumer.
// Resolved paths stay process-local; the receipt contains symbolic keys only.

import { createHash } from "node:crypto";
import { copyFile, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { join, resolve } from "node:path";
import { tmpdir } from "node:os";

const LONGHORN_BASE = "c20ac0a208840ff6424f2d334f5bc0a09ff6a07e";
const SWALLOWTAIL_SHA = "d7e93e5552c5b272e55ddef8a531b5dd32e81bf0";
const CLEAN_RUNS = 32;
const SAFE_OUTCOME =
  "provider-free-fixture:pass readiness=ready callbacks=once results=once cancellation=once replay=none redaction=safe";
const scriptRoot = resolve(import.meta.dir, "..");
const longhornRoot = resolve(requireEnvironment("AGENT_LONGHORN_REPO"));
const projectsRoot = resolve(requireEnvironment("AGENT_PROJECTS_ROOT"));
const swallowtailRoot = join(projectsRoot, "swallowtail");
const sourcePath = join(
  scriptRoot,
  "fixtures",
  "agent-tool-dispatch-provider-free",
  "main.rs",
);

const longhornHead = await cleanHead(longhornRoot, "AGENT_LONGHORN_REPO");
const swallowtailHead = await cleanHead(
  swallowtailRoot,
  "AGENT_PROJECTS_ROOT/swallowtail",
);
if (swallowtailHead !== SWALLOWTAIL_SHA) {
  throw new Error(
    `AGENT_PROJECTS_ROOT/swallowtail must be ${SWALLOWTAIL_SHA}; got ${swallowtailHead}`,
  );
}
await run(["git", "merge-base", "--is-ancestor", LONGHORN_BASE, longhornHead], longhornRoot);

const sourceGraphBefore = await sourceGraphDigests(longhornRoot, swallowtailRoot);
const first = await mkdtemp(join(tmpdir(), "longhorn-agent-tool-dispatch-"));
const second = await mkdtemp(join(tmpdir(), "longhorn-agent-tool-dispatch-repeat-"));
let receipt: Record<string, unknown> | undefined;

try {
  const firstEvidence = await prepare(first);
  const secondEvidence = await prepare(second);
  if (
    firstEvidence.manifestSha256 !== secondEvidence.manifestSha256
    || firstEvidence.lockSha256 !== secondEvidence.lockSha256
  ) {
    throw new Error("normalized manifest or lockfile hash is not reproducible");
  }

  for (let index = 0; index < CLEAN_RUNS; index += 1) {
    const outcome = (
      await run(
        ["cargo", "run", "--locked", "--quiet"],
        first,
        { CARGO_TARGET_DIR: join(first, "target") },
      )
    ).trim();
    if (outcome !== SAFE_OUTCOME) {
      throw new Error(
        `fixture run ${index + 1} returned an unexpected safe outcome: ${outcome}`,
      );
    }
  }

  const sourceGraphAfter = await sourceGraphDigests(longhornRoot, swallowtailRoot);
  if (sourceGraphBefore !== sourceGraphAfter) {
    throw new Error("a source Cargo manifest or lockfile changed during the fixture");
  }
  await cleanHead(longhornRoot, "AGENT_LONGHORN_REPO");
  await cleanHead(swallowtailRoot, "AGENT_PROJECTS_ROOT/swallowtail");

  receipt = {
    schema: "longhorn.agent-tool-dispatch-source-consumer-receipt.v1",
    outcome: "pass",
    sources: {
      AGENT_LONGHORN_REPO: longhornHead,
      "AGENT_PROJECTS_ROOT/swallowtail": swallowtailHead,
    },
    exactBase: LONGHORN_BASE,
    normalizedManifestSha256: firstEvidence.manifestSha256,
    lockfileSha256: firstEvidence.lockSha256,
    commands: {
      generateLockfile: "pass-twice-identical",
      lockedCompileRun: `pass-${CLEAN_RUNS}-consecutive`,
    },
    protocol: {
      readiness: "ready",
      preDispatchRefusal: "pass",
      callbacks: 1,
      results: 1,
      cancellations: 1,
      postTerminalReplay: 0,
      mutationReplay: 0,
      diagnostics: "redacted",
      consecutiveCleanRuns: CLEAN_RUNS,
    },
    sourceGraphsUnchanged: true,
    temporaryFixtureRemoved: true,
  };
} finally {
  await rm(first, { recursive: true, force: true });
  await rm(second, { recursive: true, force: true });
}
if (!receipt) throw new Error("fixture completed without a receipt");
console.log(JSON.stringify(receipt, null, 2));

async function prepare(directory: string) {
  const manifest = renderManifest(longhornRoot, swallowtailRoot);
  await writeFile(join(directory, "Cargo.toml"), manifest);
  await copyFile(sourcePath, join(directory, "main.rs"));
  await run(["cargo", "generate-lockfile"], directory);
  const normalized = renderManifest(
    "${AGENT_LONGHORN_REPO}",
    "${AGENT_PROJECTS_ROOT}/swallowtail",
  );
  const lock = await readFile(join(directory, "Cargo.lock"));
  return {
    manifestSha256: sha256(normalized),
    lockSha256: sha256(lock),
  };
}

function renderManifest(longhorn: string, swallowtail: string): string {
  const path = (value: string) => value.replaceAll("\\", "\\\\").replaceAll('"', '\\"');
  return `[package]\nname = "longhorn-agent-tool-dispatch-provider-free-fixture"\nversion = "0.0.0"\nedition = "2024"\nrust-version = "1.95"\npublish = false\n\n[workspace]\n\n[dependencies]\nlonghorn-agent-tool-dispatch = { path = "${path(join(longhorn, "crates", "longhorn-agent-tool-dispatch"))}" }\nswallowtail-core = { path = "${path(join(swallowtail, "crates", "swallowtail-core"))}" }\nswallowtail-host-local = { path = "${path(join(swallowtail, "crates", "swallowtail-host-local"))}" }\nswallowtail-runtime = { path = "${path(join(swallowtail, "crates", "swallowtail-runtime"))}" }\nswallowtail-testkit = { path = "${path(join(swallowtail, "crates", "swallowtail-testkit"))}" }\n\n[[bin]]\nname = "provider-free-fixture"\npath = "main.rs"\n`;
}

async function cleanHead(repository: string, symbolic: string): Promise<string> {
  const status = await run(["git", "status", "--porcelain"], repository);
  if (status.trim() !== "") {
    throw new Error(`${symbolic} must be clean`);
  }
  return (await run(["git", "rev-parse", "HEAD"], repository)).trim();
}

async function sourceGraphDigests(longhorn: string, swallowtail: string): Promise<string> {
  const paths = [
    join(longhorn, "Cargo.toml"),
    join(longhorn, "Cargo.lock"),
    join(swallowtail, "Cargo.toml"),
    join(swallowtail, "Cargo.lock"),
  ];
  const parts = await Promise.all(paths.map((path) => readFile(path)));
  return sha256(Buffer.concat(parts));
}

function sha256(value: string | Uint8Array): string {
  return createHash("sha256").update(value).digest("hex");
}

function requireEnvironment(key: string): string {
  const value = process.env[key];
  if (!value) throw new Error(`${key} is required`);
  return value;
}

async function run(
  command: string[],
  cwd: string,
  environment: Record<string, string> = {},
): Promise<string> {
  const subprocess = Bun.spawn(command, {
    cwd,
    env: { ...process.env, ...environment },
    stdout: "pipe",
    stderr: "pipe",
  });
  const [stdout, stderr, exitCode] = await Promise.all([
    new Response(subprocess.stdout).text(),
    new Response(subprocess.stderr).text(),
    subprocess.exited,
  ]);
  if (exitCode !== 0) {
    throw new Error(`${command.join(" ")} failed\n${stdout}\n${stderr}`);
  }
  return stdout;
}
