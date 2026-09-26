// Fail when a `release:gates` command needs a tool `release.yml` does not install.
//
// 0.2.2 dry run 36254788295 died on `cargo deny`: the advisories gate needs
// cargo-deny, the runner does not ship it, and local qa could not see that
// because developer machines often have the plugin. Installing the crate in
// the workflow is the runner fix; this check is the local one.
//
// Tools are derived from gate commands (`cargo deny` → cargo-deny), not a
// hand-kept required-tool list. A comment or step name is not an install.

import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

import { parseReleaseGates } from "./check-release-gates-alignment.ts";

const defaultRepoRoot = resolve(import.meta.dir, "..");
const WORKFLOW = ".github/workflows/release.yml";

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

export type ToolNeed = {
  readonly tool: string;
  readonly reason: string;
};

export type Requirement = {
  readonly gate: string;
  readonly command: string;
  readonly tool: string;
  readonly provided: boolean;
  readonly via: string | null;
};

export type ReleaseRunnerToolsResult = {
  readonly requirements: readonly Requirement[];
  readonly failures: readonly Requirement[];
};

export type WorkflowInstalls = {
  readonly tools: ReadonlySet<string>;
  readonly via: ReadonlyMap<string, string>;
};

export function tokenize(input: string): string[] {
  const tokens: string[] = [];
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

export function toolsNeededByCommand(command: string): ToolNeed[] {
  const trimmed = command.trim();
  if (trimmed.length === 0) {
    throw new Error("cannot map an empty gate command to a runner tool");
  }
  const needs: ToolNeed[] = [];
  const seen = new Set<string>();
  const add = (tool: string, reason: string) => {
    if (seen.has(tool)) return;
    seen.add(tool);
    needs.push({ tool, reason });
  };

  for (const subcommand of cargoSubcommands(trimmed)) {
    if (STOCK_CARGO_SUBCOMMANDS.has(subcommand)) {
      add("cargo", `stock cargo ${subcommand} in \`${trimmed}\``);
      continue;
    }
    add("cargo", `cargo plugin \`${subcommand}\` in \`${trimmed}\``);
    add(`cargo-${subcommand}`, `cargo plugin from \`${trimmed}\``);
  }

  const argv = leadingCommand(tokenize(trimmed));
  const program = argv[0];
  if (!program) {
    throw new Error(`cannot map gate command to a tool without executing it: ${trimmed}`);
  }
  if ((program === "sh" || program === "bash" || program === "zsh") && argv[1] === "-c" && needs.length === 0) {
    throw new Error(`cannot map gate command to a tool without executing it: ${trimmed}`);
  }
  if (program !== "cargo" && !STOCK_RUNNER_COMMANDS.has(program)) {
    add(program, `command \`${trimmed}\``);
  } else if (program !== "cargo" && needs.length === 0) {
    add(program, `stock runner command \`${trimmed}\``);
  }
  if (needs.length === 0) {
    throw new Error(`cannot map gate command to a tool without executing it: ${trimmed}`);
  }
  return needs;
}

export function parseWorkflowInstalls(yaml: string): WorkflowInstalls {
  const via = new Map<string, string>();
  const add = (tool: string, source: string) => {
    if (!via.has(tool)) via.set(tool, source);
  };
  for (const uses of workflowUses(yaml)) {
    const action = uses.split("@")[0] ?? uses;
    for (const tool of toolsProvidedByAction(uses)) {
      add(tool, action);
    }
  }
  for (const script of workflowRunScripts(yaml)) {
    for (const crate of cargoInstallCrates(script)) {
      add(crate, `cargo install ${crate}`);
    }
  }
  return { tools: new Set(via.keys()), via };
}

export function checkReleaseRunnerTools(repoRoot = defaultRepoRoot): ReleaseRunnerToolsResult {
  const releaseToml = readFileSync(join(repoRoot, "config/release.toml"), "utf8");
  const workflow = readFileSync(join(repoRoot, WORKFLOW), "utf8");
  const installs = parseWorkflowInstalls(workflow);
  const requirements: Requirement[] = [];
  for (const gate of parseReleaseGates(releaseToml)) {
    if (gate.name === "workspace") continue;
    for (const need of toolsNeededByCommand(gate.command)) {
      const via = howProvided(installs, need.tool);
      requirements.push({
        gate: gate.name,
        command: gate.command,
        tool: need.tool,
        provided: via !== null,
        via,
      });
    }
  }
  return {
    requirements,
    failures: requirements.filter((item) => !item.provided),
  };
}

export function formatReleaseRunnerToolsFailure(result: ReleaseRunnerToolsResult): string {
  const lines = result.failures.map(
    (item) => `  ${item.gate}: ${item.tool}  (from \`${item.command}\`)`,
  );
  return (
    `Release gates need tools the release runner does not install:\n\n${lines.join("\n")}\n\n` +
    `${WORKFLOW} must install each named tool. A comment or step name is not\n` +
    "an install. Installing the tool on the runner only hides the next gap."
  );
}

export function workflowUses(yaml: string): string[] {
  const uses: string[] = [];
  for (const raw of yaml.split("\n")) {
    const line = stripYamlComment(raw);
    const match = /^\s+-?\s*uses:\s+(\S+)/.exec(line);
    if (match?.[1]) uses.push(match[1]);
  }
  return uses;
}

export function workflowRunScripts(yaml: string): string[] {
  const scripts: string[] = [];
  const lines = yaml.split("\n");
  for (let index = 0; index < lines.length; ) {
    const raw = lines[index]!;
    const line = stripYamlComment(raw);
    const match = /^(\s*)(?:-\s+)?run:\s*(.*)$/.exec(line);
    if (!match) {
      index += 1;
      continue;
    }
    const indent = match[1].length;
    const rest = match[2].trim();
    if (/^(?:\||>)[-+]?\s*$/.test(rest)) {
      index += 1;
      const body: string[] = [];
      while (index < lines.length) {
        const next = lines[index]!;
        if (next.trim().length === 0) {
          body.push("");
          index += 1;
          continue;
        }
        const nextIndent = next.search(/\S/);
        if (nextIndent !== -1 && nextIndent <= indent) break;
        body.push(stripYamlComment(next).trimEnd());
        index += 1;
      }
      scripts.push(body.join("\n"));
      continue;
    }
    if (rest.length > 0) scripts.push(rest);
    index += 1;
  }
  return scripts;
}

export function cargoInstallCrates(script: string): string[] {
  const crates: string[] = [];
  const joined = script.replace(/\\\n/g, " ");
  for (const raw of joined.split("\n")) {
    const argv = leadingCommand(tokenize(stripYamlComment(raw).trim()));
    if (argv[0] !== "cargo" || argv[1] !== "install") continue;
    for (let index = 2; index < argv.length; ) {
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

function toolsProvidedByAction(uses: string): string[] {
  const action = (uses.split("@")[0] ?? uses).split("/")[1] ?? uses;
  if (action === "rust-toolchain") return ["cargo", "rustc", "rustup"];
  if (action === "setup-node") return ["node", "npm", "npx"];
  if (action.startsWith("setup-")) return [action.slice("setup-".length)];
  return [];
}

function howProvided(installs: WorkflowInstalls, tool: string): string | null {
  if (STOCK_RUNNER_COMMANDS.has(tool)) return "stock runner";
  return installs.via.get(tool) ?? null;
}

function cargoSubcommands(command: string): string[] {
  const subcommands: string[] = [];
  const pattern = /\bcargo\b/g;
  let match: RegExpExecArray | null;
  while ((match = pattern.exec(command))) {
    const rest = command.slice(match.index + match[0].length);
    const tokens = tokenize(rest);
    const subcommand = tokens.find((token) => !token.startsWith("+") && !token.startsWith("-"));
    if (subcommand) subcommands.push(subcommand);
  }
  return subcommands;
}

function leadingCommand(argv: string[]): string[] {
  let index = 0;
  while (index < argv.length && /^[A-Za-z_][A-Za-z0-9_]*=/.test(argv[index]!)) {
    index += 1;
  }
  return argv.slice(index);
}

function stripYamlComment(line: string): string {
  return line.replace(/(^|\s)#.*$/, "$1");
}
