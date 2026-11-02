// Bundled local child-view source: disposable native consumer check and proof
// (contract 017, native-content local selectors).
//
// The consumer lives under `examples/native-content-local-consumer/`, is not
// a root workspace member, and carries no product value. This script stages
// the child-view crate, its two shared dependencies and the consumer into a
// disposable private workspace, then:
//
// - `--check-only`: compiles the consumer (`cargo check --all-targets`) and
//   runs `cargo fmt --check` on it. This is `check:native-content:local`.
// - default: builds and runs the real macOS application, which attaches one
//   bundled local document through the public API and prints one
//   `ncl-proof-report:{json}` line. This is `proof:native-content:local`.
//
// The proof runs a real WKWebView in a real window, so it needs a macOS host
// with a GUI session. It runs the app binary with a hard timeout beyond the
// app's own 120 s watchdog, so a stuck native layer fails the proof instead
// of hanging the selector.
//
// Every Cargo route runs through the switch point: checkout-level package
// inventories through `rustCommand`, the staged workspace through one
// `rustExec` lease with private final outputs on external build scratch.
// Keep the stage with KEEP_NATIVE_CONTENT_LOCAL=1.

import { spawnSync } from "node:child_process";
import { cp, lstat, mkdtemp, readdir, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import {
  prepareStagedWorkspace,
  privateCargoCommand,
  rustCommand,
  rustExec,
  taskScratch,
} from "./rust-command.ts";
import { MSRV_TOOLCHAIN } from "./msrv.ts";

const repoRoot = resolve(import.meta.dir, "..");
const consumerName = "longhorn-native-content-local-consumer";
const exampleRoot = join(repoRoot, "examples/native-content-local-consumer");
const consumerRoot = join(exampleRoot, "consumer");
const checkOnly = process.argv.includes("--check-only");

/**
 * The crates the proof stages. The child-view crate is the changed public
 * surface; the other two are the only crates it depends on, so the staged
 * graph stays exactly the graph a bundled-local consumer needs.
 */
const crates = [
  "longhorn-core",
  "longhorn-native-content",
  "longhorn-tauri-native-content-child-view",
] as const;

if (process.platform !== "darwin") {
  console.error(
    `native-content:local runs on macOS hosts only; this host is ${process.platform}.`,
  );
  process.exit(1);
}

const stage = await mkdtemp(join(tmpdir(), "longhorn-native-content-local-"));
// Private final Cargo outputs for the disposable consumer, on external build
// scratch. The intermediates keep the build directory the exec lease supplies.
const buildScratch = await taskScratch("longhorn-native-content-local");
const stagedBinary = join(
  buildScratch.root,
  "debug",
  consumerName,
);

try {
  await verifyExampleSources();
  const inventories = crates.map((name) => verifyPackageInventory(name));

  for (const name of crates) {
    await cp(join(repoRoot, "crates", name), join(stage, "crates", name), { recursive: true });
  }
  await cp(consumerRoot, join(stage, "consumer"), { recursive: true });
  await writeFile(join(stage, "Cargo.toml"), await privateWorkspaceManifest());
  await cp(join(repoRoot, "Cargo.lock"), join(stage, "Cargo.lock"));

  // One exec lease over the staged workspace at the MSRV pin, with private
  // final outputs on build scratch: the tool inside calls plain Cargo. The
  // real `target/` directory keeps the lease adoption-free; see
  // `prepareStagedWorkspace`.
  await prepareStagedWorkspace(stage);
  const staged = (args: readonly string[]) =>
    rustExec(privateCargoCommand(buildScratch.root, args), {
      manifestPath: join(stage, "Cargo.toml"),
      toolchain: MSRV_TOOLCHAIN,
    });

  if (checkOnly) {
    run(staged(["cargo", "fmt", "--check", "-p", consumerName]), stage);
    run(staged(["cargo", "check", "-p", consumerName, "--all-targets", "--offline", "--quiet"]), stage);
    console.log(
      JSON.stringify(
        {
          schema: "longhorn.native-content-local-consumer-check.v1",
          outcome: "pass",
          platform: process.platform,
          toolchain: MSRV_TOOLCHAIN,
          crates: [...crates],
          packages: inventories,
          consumer: consumerName,
          proofLimits: [
            "Compiles the staged consumer; it does not run the native application",
            "macOS hosts only",
          ],
        },
        null,
        2,
      ),
    );
  } else {
    run(staged(["cargo", "build", "-p", consumerName, "--offline", "--quiet"]), stage);
    const startedAt = performance.now();
    // The app carries its own 120 s watchdog; this hard timeout only stops a
    // wedged native layer. A missing GUI session fails here, loudly.
    const app = spawnSync(stagedBinary, {
      encoding: "utf8",
      timeout: 200_000,
      killSignal: "SIGTERM",
      env: { ...process.env },
      maxBuffer: 16 * 1024 * 1024,
    });
    const seconds = Number(((performance.now() - startedAt) / 1000).toFixed(1));
    const stdout = app.stdout ?? "";
    const reportLine = stdout
      .split("\n")
      .find((line) => line.startsWith("ncl-proof-report:"));
    if (reportLine === undefined) {
      throw new Error(
        `the native consumer printed no proof report (exit ${app.status}, signal ${app.signal})\n${stdout}\n${app.stderr ?? ""}`,
      );
    }
    const report = JSON.parse(reportLine.slice("ncl-proof-report:".length));
    if (report.schema !== "longhorn.native-content-local-proof.v1") {
      throw new Error(`unexpected proof report schema: ${report.schema}`);
    }
    if (report.outcome !== "pass" || app.status !== 0) {
      throw new Error(
        `the native proof failed (outcome ${report.outcome}, exit ${app.status})\n${reportLine}`,
      );
    }

    // The evidence must show exactly the claims the proof exists for.
    assertClaim(report.steps, "observed-source-identity");
    assertClaim(report.steps, "adapter-typed-escape-denial");
    assertClaim(report.steps, "document-title-evidence");
    assertClaim(report.steps, "detach-applied");
    assertClaim(report.steps, "post-detach-observation");
    assertClaim(report.steps, "teardown-receipt");
    const pageLoadUrls = report.pageLoadUrls as string[];
    if (pageLoadUrls.length === 0) {
      throw new Error("the proof recorded no page-load events");
    }
    for (const url of pageLoadUrls) {
      if (!url.startsWith("tauri://localhost/")) {
        throw new Error(`a page-load event left the application asset protocol: ${url}`);
      }
    }
    const ipcWebviews = report.ipc.webviews as string[];
    if (JSON.stringify(ipcWebviews) !== JSON.stringify(["proof-child"])) {
      throw new Error(`the scoped IPC receipt came from unexpected webviews: ${ipcWebviews}`);
    }
    if (report.ipc.nonceMatchesInjection !== true) {
      throw new Error("the injected initialization-script nonce did not round-trip");
    }

    console.log(
      JSON.stringify(
        {
          schema: "longhorn.native-content-local-proof.v1",
          outcome: "pass",
          platform: report.platform,
          tauriVersion: report.tauriVersion,
          toolchain: MSRV_TOOLCHAIN,
          crates: [...crates],
          packages: inventories,
          consumer: consumerName,
          runSeconds: seconds,
          source: report.source,
          navigationPolicy: report.navigationPolicy,
          csp: report.csp,
          capability: report.capability,
          observedSourceUrl: report.steps.find(
            (step: { step: string }) => step.step === "observed-source-identity",
          )?.detail.url,
          documentTitle: report.documentTitle,
          proofLimits: [
            "macOS with a GUI session: the proof runs a real WKWebView child in a real window",
            "The observed tauri://localhost spelling is Tauri's macOS asset-protocol mapping, recorded as evidence, not a Longhorn promise",
            "The staged consumer is disposable: product-free labels, islands and commands",
          ],
          nativeReport: report,
        },
        null,
        2,
      ),
    );
  }
} catch (error) {
  console.error(String(error instanceof Error ? error.stack ?? error.message : error));
  process.exitCode = 1;
} finally {
  if (process.env.KEEP_NATIVE_CONTENT_LOCAL === "1") {
    console.error(`retained native-content local stage: ${stage}`);
  } else {
    await rm(stage, { recursive: true, force: true });
  }
  await buildScratch.dispose();
}

function assertClaim(steps: ReadonlyArray<{ step: string }>, name: string): void {
  if (!steps.some((step) => step.step === name)) {
    throw new Error(`the native proof never recorded the ${name} step`);
  }
}

/**
 * The consumer must stay a template, not a workspace member, and must carry
 * no product vocabulary. Both are what make the proof a second consumer.
 */
async function verifyExampleSources(): Promise<void> {
  const root = await readFile(join(repoRoot, "Cargo.toml"), "utf8");
  if (root.includes("native-content-local-consumer")) {
    throw new Error("the native-content local consumer must not be a root workspace member");
  }

  const manifest = await readFile(join(consumerRoot, "Cargo.toml"), "utf8");
  const declared = [...manifest.matchAll(/^(longhorn-[\w-]+)(?:\.workspace)?\s*=/gm)].map(
    (match) => match[1]!,
  );
  for (const name of declared) {
    if (!(crates as readonly string[]).includes(name)) {
      throw new Error(`the consumer depends on ${name}, which is not a staged crate`);
    }
  }
  if (!declared.includes("longhorn-tauri-native-content-child-view")) {
    throw new Error("the consumer does not depend on the child-view crate");
  }

  // Longhorn's own names are fine; another repository's product vocabulary is
  // not, because the consumer must not copy a donor's product policy.
  const productVocabulary = /acowtancy|bovine|praxis|jetstream|loophole|nucleus|soundcheck|figmatic/i;
  const offenders: string[] = [];
  for (const relative of await readdir(consumerRoot, { recursive: true })) {
    if (!(await lstat(join(consumerRoot, relative))).isFile()) continue;
    for (const [index, line] of (await readFile(join(consumerRoot, relative), "utf8")).split("\n").entries()) {
      if (productVocabulary.test(line)) {
        offenders.push(`${relative}:${index + 1}: ${line.trim()}`);
      }
    }
  }
  if (offenders.length > 0) {
    throw new Error(`the native-content local consumer carries product vocabulary:\n${offenders.join("\n")}`);
  }
}

/**
 * `cargo package --list` is the packaged inventory: what a consumer taking
 * the crate by tag actually receives. The proof copies crate sources, so this
 * is the check that those sources are the ones a package ships.
 */
function verifyPackageInventory(name: string): { name: string; files: number } {
  const listing = run(
    rustCommand(["package", "-p", name, "--list", "--allow-dirty"]),
    repoRoot,
  );
  const files = listing.trim().split("\n");
  for (const required of ["Cargo.toml"]) {
    if (!files.includes(required)) throw new Error(`${name}'s package inventory omits ${required}`);
  }
  if (!files.some((file) => file.startsWith("src/"))) {
    throw new Error(`${name}'s package inventory omits its sources`);
  }
  return { name, files: files.length };
}

/**
 * The private workspace: only the staged crates and the consumer, with the
 * repository's `[workspace.package]`, dependency versions and lints. Every
 * `longhorn-*` dependency line outside the selected set is dropped, so
 * nothing the consumer builds can reach a crate this proof does not stage.
 */
async function privateWorkspaceManifest(): Promise<string> {
  const root = await readFile(join(repoRoot, "Cargo.toml"), "utf8");
  const tail = root.slice(root.indexOf("[workspace.package]"));
  const version = /\nversion = "([^"]+)"/.exec(tail)?.[1];
  if (!version) throw new Error("the workspace package version is missing");
  const selected = new Set<string>(crates);
  const filtered = tail
    .split("\n")
    .filter((line) => {
      const match = line.match(/^(longhorn-[\w-]+)\s*=/);
      return match === null || selected.has(match[1]!);
    })
    .join("\n");
  const members = [
    ...crates.map((name) => `  "crates/${name}",`),
    '  "consumer",',
  ].join("\n");
  return `[workspace]\nmembers = [\n${members}\n]\nresolver = "2"\n\n${filtered}`;
}

function run(command: readonly string[], cwd: string): string {
  // `scripts/rust-command.sh` feeds every command here: the checkout-level
  // `package --list` inventories and the staged workspace's `oxide exec`
  // lease, whose tool calls plain Cargo.
  const result = spawnSync(command[0]!, command.slice(1), {
    cwd,
    encoding: "utf8",
    env: { ...process.env },
    maxBuffer: 64 * 1024 * 1024,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(
      `${command.join(" ")} failed in ${cwd} (exit ${result.status})\n${result.stdout}\n${result.stderr}`,
    );
  }
  return result.stdout;
}
