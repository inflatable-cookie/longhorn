// Fails when scripts invoke tools a clean GitHub runner does not have, when a
// release gate needs a tool `release.yml` does not install, or when the Rust
// CI jobs lack the pinned Bun their tests require.
//
// Release run 6 died on missing `rg` twice: once in the greenfield proof, once
// in check-release-floor.sh, where `set -o pipefail` made the absence look
// like a missing MSRV toolchain. Local machines usually have ripgrep, so qa
// never saw it. 0.2.2 dry run 36254788295 died the same way on `cargo deny`.
//
// Comments may mention rg. Command invocations may not. A workflow comment
// naming cargo-deny is not an install.

import { readFileSync } from "node:fs";
import { readdir, readFile } from "node:fs/promises";
import { extname, join, relative, resolve } from "node:path";

import { parseReleaseGates } from "./check-release-gates-alignment.ts";

const repoRoot = resolve(import.meta.dir, "..");
const RELEASE_WORKFLOW = ".github/workflows/release.yml";
const CI_WORKFLOW = ".github/workflows/ci.yml";

const SKIPPED = new Set([
  "node_modules",
  ".git",
  "target",
  "dist",
  "build",
  ".svelte-kit",
  ".vite",
]);
const ROOTS = ["scripts", ".github", "config"];
const EXTENSIONS = new Set([".ts", ".tsx", ".js", ".mjs", ".cjs", ".sh", ".yml", ".yaml", ".toml"]);

// Word-boundary forms that are invocations, not prose. Comments are stripped
// before matching so "prefer grep, not rg" in a header stays allowed.
const FORBIDDEN = [
  { tool: "rg", pattern: /(?:^|[\s|`$])rg(?:\s|["'`|]|$)/ },
  { tool: "ripgrep", pattern: /(?:^|[\s|`$])ripgrep(?:\s|["'`|]|$)/ },
];

// Built-in cargo plus rustup components the toolchain action provides.
// Plugins (`deny`, `audit`, `nextest`) are not in this set: they need
// `cargo install cargo-<subcommand>` in a workflow `run:` script.
const STOCK_CARGO_SUBCOMMANDS = new Set([
  "add",
  "bench",
  "build",
  "check",
  "clean",
  "clippy",
  "config",
  "doc",
  "fetch",
  "fix",
  "fmt",
  "generate-lockfile",
  "help",
  "info",
  "init",
  "install",
  "locate-project",
  "login",
  "logout",
  "metadata",
  "new",
  "owner",
  "package",
  "pkgid",
  "publish",
  "read-manifest",
  "remove",
  "report",
  "run",
  "rustc",
  "rustdoc",
  "search",
  "test",
  "tree",
  "uninstall",
  "update",
  "vendor",
  "verify-project",
  "version",
  "yank",
]);

// First-token programs a GitHub macos runner already has. Not bun, effigy,
// cargo, or cargo plugins: those come from workflow setup / install steps.
const STOCK_RUNNER_COMMANDS = new Set(["bash", "git", "grep", "sh", "zsh"]);

const CARGO_INSTALL_FLAGS_WITH_VALUE = new Set([
  "--branch",
  "--config",
  "--example",
  "--features",
  "--git",
  "--index",
  "--path",
  "--profile",
  "--registry",
  "--rev",
  "--root",
  "--tag",
  "--target",
  "--version",
  "-C",
  "-Z",
]);

function tokenize(input: string): Array<string> {
  const tokens: Array<string> = [];
  const pattern = /(?:'[^']*'|"[^"]*"|\S+)/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(input))) {
    let token = match[0];
    if (
      (token.startsWith("'") && token.endsWith("'") && token.length >= 2) ||
      (token.startsWith('"') && token.endsWith('"') && token.length >= 2)
    ) {
      token = token.slice(1, -1);
    }
    tokens.push(token);
  }
  return tokens;
}

export function toolsNeededByCommand(command: string): Array<string> {
  const trimmed = command.trim();
  if (trimmed.length === 0) {
    throw new Error("cannot map an empty gate command to a runner tool");
  }
  const needs = new Set<string>();

  for (const subcommand of cargoSubcommands(trimmed)) {
    needs.add("cargo");
    if (!STOCK_CARGO_SUBCOMMANDS.has(subcommand)) {
      needs.add(`cargo-${subcommand}`);
    }
  }

  const argv = leadingCommand(tokenize(trimmed));
  const program = argv[0];
  if (!program) {
    throw new Error(`cannot map gate command to a tool without executing it: ${trimmed}`);
  }
  if (
    (program === "sh" || program === "bash" || program === "zsh") &&
    argv[1] === "-c" &&
    needs.size === 0
  ) {
    throw new Error(`cannot map gate command to a tool without executing it: ${trimmed}`);
  }
  if (program !== "cargo" && (!STOCK_RUNNER_COMMANDS.has(program) || needs.size === 0)) {
    needs.add(program);
  }
  if (needs.size === 0) {
    throw new Error(`cannot map gate command to a tool without executing it: ${trimmed}`);
  }
  return [...needs];
}

export function cargoInstallCrates(script: string): Array<string> {
  const crates: Array<string> = [];
  for (const line of commandLines(script)) {
    const argv = leadingCommand(tokenize(line));
    if (argv[0] !== "cargo" || argv[1] !== "install") {
      continue;
    }
    for (let index = 2; index < argv.length;) {
      const token = argv[index]!;
      if (token.startsWith("-")) {
        const flag = token.split("=")[0]!;
        if (token.includes("=") || !CARGO_INSTALL_FLAGS_WITH_VALUE.has(flag)) {
          index += 1;
        } else {
          index += 2;
        }
        continue;
      }
      crates.push(token);
      index += 1;
    }
  }
  return crates;
}

function toolsProvidedByAction(uses: string): Array<string> {
  const action = (uses.split("@")[0] ?? uses).split("/")[1] ?? uses;
  if (action === "rust-toolchain") {
    return ["cargo", "rustc", "rustup"];
  }
  if (action === "setup-node") {
    return ["node", "npm", "npx"];
  }
  if (action.startsWith("setup-")) {
    return [action.slice("setup-".length)];
  }
  return [];
}

function cargoSubcommands(command: string): Array<string> {
  const subcommands: Array<string> = [];
  // A gate that launches Cargo through `scripts/rust-command.sh` runs `cargo`
  // on a CI runner and `oxide` on a developer machine, so the word `cargo` may
  // not appear at all. The runner tool is the same either way, so read the
  // subcommand after the launcher exactly as after a literal `cargo`.
  const pattern = /\bcargo\b|scripts\/rust-command\.sh/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(command))) {
    const rest = command.slice(match.index + match[0].length);
    const tokens = tokenize(rest);
    const subcommand = tokens.find((token) => !token.startsWith("+") && !token.startsWith("-"));
    if (subcommand) {
      subcommands.push(subcommand);
    }
  }
  return subcommands;
}

function leadingCommand(argv: Array<string>): Array<string> {
  let index = 0;
  while (index < argv.length && /^[A-Za-z_][A-Za-z0-9_]*=/.test(argv[index]!)) {
    index += 1;
  }
  return argv.slice(index);
}

type WorkflowStep = {
  conditional: boolean;
  uses: string | null;
  run: string | null;
  bunVersion: string | null;
};

function isMapping(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function workflowJobs(source: string): Map<string, Array<WorkflowStep>> {
  const workflow = Bun.YAML.parse(source);
  if (!isMapping(workflow) || !isMapping(workflow.jobs)) {
    throw new Error("workflow must declare a jobs mapping");
  }

  const jobs = new Map<string, Array<WorkflowStep>>();
  for (const [job, config] of Object.entries(workflow.jobs)) {
    if (!isMapping(config)) {
      throw new Error(`invalid workflow job ${job}`);
    }
    if ("uses" in config) {
      continue;
    } // Reusable workflow, no local steps.
    const steps = config.steps;
    if (!Array.isArray(steps)) {
      throw new Error(`workflow job ${job} must declare steps`);
    }
    jobs.set(
      job,
      steps.map((step: unknown) => {
        if (!isMapping(step)) {
          throw new Error(`invalid step in workflow job ${job}`);
        }
        const version = isMapping(step.with) ? step.with["bun-version"] : null;
        return {
          conditional: "if" in step,
          uses: typeof step.uses === "string" ? step.uses : null,
          run: typeof step.run === "string" ? step.run : null,
          bunVersion: typeof version === "string" ? version : null,
        };
      }),
    );
  }
  return jobs;
}

function stripShellComment(line: string): string {
  // A shell comment starts at a token boundary outside single or double quotes.
  return line.replaceAll(
    /(?:'[^']*'|"[^"]*"|\\.|[^\s#][^\s]*)|(^|\s)#.*$/g,
    (match, space) => space ?? match,
  );
}

function commandLines(script: string): Array<string> {
  return script
    .replaceAll(/\\\r?\n/g, " ")
    .split("\n")
    .map((line) => stripShellComment(line).trim());
}

export function releaseWorkflowInstalls(workflow: string): Map<string, string> {
  const gateJobs: Array<Map<string, string>> = [];
  for (const steps of workflowJobs(workflow).values()) {
    const installs = new Map<string, string>();
    scan: for (const step of steps) {
      if (!step.conditional && step.uses) {
        const action = step.uses.split("@", 1).join("");
        for (const tool of toolsProvidedByAction(step.uses)) {
          installs.set(tool, action);
        }
      }
      for (const line of commandLines(step.run ?? "")) {
        const argv = leadingCommand(tokenize(line));
        if (argv[0] === "effigy" && argv.includes("release:gates")) {
          gateJobs.push(installs);
          break scan;
        }
        if (step.conditional) {
          continue;
        }
        for (const crate of cargoInstallCrates(line)) {
          installs.set(crate, `cargo install ${crate}`);
        }
      }
    }
  }

  const [first, ...remaining] = gateJobs;
  if (!first) {
    throw new Error(`${RELEASE_WORKFLOW} has no step that runs effigy release:gates`);
  }
  return new Map([...first].filter(([tool]) => remaining.every((job) => job.has(tool))));
}

export function checkReleaseRunnerTools(root = repoRoot) {
  const gates = parseReleaseGates(readFileSync(join(root, "config/release.toml"), "utf8"));
  const installs = releaseWorkflowInstalls(readFileSync(join(root, RELEASE_WORKFLOW), "utf8"));
  const requirements = gates
    .filter(({ name }) => name !== "workspace")
    .flatMap(({ name, command }) =>
      toolsNeededByCommand(command).map((tool) => ({
        gate: name,
        command,
        tool,
        via: STOCK_RUNNER_COMMANDS.has(tool) ? "stock runner" : (installs.get(tool) ?? null),
      })),
    );
  return { requirements, failures: requirements.filter(({ via }) => via === null) };
}

type ReleaseRunnerToolsResult = ReturnType<typeof checkReleaseRunnerTools>;

export function formatReleaseRunnerToolsFailure({ failures }: ReleaseRunnerToolsResult): string {
  const lines = failures.map(
    ({ gate, tool, command }) => `  ${gate}: ${tool}  (from \`${command}\`)`,
  );
  return [
    "Release gates need tools the release runner does not install:",
    ...lines,
    `Install each tool in an unconditional step of the same job in ${RELEASE_WORKFLOW}, before effigy release:gates.`,
  ].join("\n");
}

const BUN_ACTION = "oven-sh/setup-bun@";
const PINNED_BUN_ACTION = /^oven-sh\/setup-bun@[0-9a-f]{40}$/;

export function checkCiRunnerTools(workflow: string): Array<string> {
  const jobs = workflowJobs(workflow);
  const bunBeforeEffigy = (job: string) => {
    const steps = jobs.get(job) ?? [];
    const bun = steps.findIndex(({ uses }) => uses?.startsWith(BUN_ACTION));
    const effigy = steps.findIndex(({ run }) =>
      commandLines(run ?? "").some((line) => leadingCommand(tokenize(line))[0] === "effigy"),
    );
    const step = steps[bun];
    return step && !step.conditional && bun < effigy ? step : undefined;
  };

  const clients = bunBeforeEffigy("clients");
  if (!clients?.uses || !PINNED_BUN_ACTION.test(clients.uses) || !clients.bunVersion) {
    return ["clients must install SHA-pinned Bun before its first Effigy step"];
  }

  const failures: Array<string> = [];
  for (const job of ["rust", "rust-floor"]) {
    const setup = bunBeforeEffigy(job);
    if (!setup) {
      failures.push(`${job} must install Bun before its first Effigy step`);
    } else if (setup.uses !== clients.uses || setup.bunVersion !== clients.bunVersion) {
      failures.push(`${job} must use the same pinned Bun action and version as clients`);
    }
  }
  return failures;
}

type Finding = { file: string; line: number; tool: string; text: string };

export function findForbiddenTools(file: string, source: string): Array<Finding> {
  const findings: Array<Finding> = [];
  for (const [index, line] of source.split("\n").entries()) {
    const code = /\.(?:sh|ya?ml|toml)$/.test(file)
      ? stripShellComment(line)
      : line.replace(/\/\/.*$/, "").replaceAll(/\/\*.*?\*\//g, "");
    for (const { tool, pattern } of FORBIDDEN) {
      if (pattern.test(code)) {
        findings.push({ file, line: index + 1, tool, text: line.trim() });
      }
    }
  }
  return findings;
}

async function scan(root: string, directory: string): Promise<Array<Finding>> {
  const findings: Array<Finding> = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (SKIPPED.has(entry.name)) {
      continue;
    }
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      findings.push(...(await scan(root, path)));
    } else if (path !== import.meta.path && EXTENSIONS.has(extname(path))) {
      findings.push(...findForbiddenTools(relative(root, path), await readFile(path, "utf8")));
    }
  }
  return findings;
}

if (import.meta.main) {
  const findings = (
    await Promise.all(ROOTS.map((root) => scan(repoRoot, join(repoRoot, root))))
  ).flat();
  findings.push(
    ...findForbiddenTools("effigy.toml", await readFile(join(repoRoot, "effigy.toml"), "utf8")),
  );
  const release = checkReleaseRunnerTools();
  const ciFailures = checkCiRunnerTools(await readFile(join(repoRoot, CI_WORKFLOW), "utf8"));

  if (findings.length > 0 || release.failures.length > 0 || ciFailures.length > 0) {
    for (const { file, line, tool, text } of findings) {
      console.error(
        `${file}:${line} invokes '${tool}', which clean runners lack. Use grep or scan in-process.\n  ${text}`,
      );
    }
    if (release.failures.length > 0) {
      console.error(formatReleaseRunnerToolsFailure(release));
    }
    for (const failure of ciFailures) {
      console.error(`${CI_WORKFLOW}: ${failure}`);
    }
    process.exitCode = 1;
  } else {
    console.log(
      JSON.stringify(
        {
          schema: "longhorn.runner-tools.v1",
          outcome: "pass",
          forbidden: FORBIDDEN.map(({ tool }) => tool),
          releaseGateTools: release.requirements.map(({ gate, tool, via }) => ({
            gate,
            tool,
            via,
          })),
        },
        null,
        2,
      ),
    );
  }
}
