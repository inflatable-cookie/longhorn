import { requireValue } from "./proof-support.ts";
// Fails when scripts invoke tools a clean GitHub runner does not have, when a
// release gate needs a tool `release.yml` does not install, or when the Rust
// CI jobs lack the pinned Bun their tests require. A release gate is followed
// through the effigy task it runs and the shell scripts that task invokes, so a
// tool used inside a gate counts as much as the gate's own command line.
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
// cargo, or cargo plugins: those come from workflow setup / install steps. The
// base utilities are the macOS system tools every hosted image ships.
const STOCK_RUNNER_COMMANDS: ReadonlySet<string> = new Set([
  "awk",
  "basename",
  "bash",
  "cat",
  "chmod",
  "cp",
  "cut",
  "date",
  "diff",
  "dirname",
  "env",
  "find",
  "git",
  "grep",
  "head",
  "jq",
  "ln",
  "ls",
  "mkdir",
  "mktemp",
  "mv",
  "nice",
  "readlink",
  "rm",
  "sed",
  "sh",
  "sleep",
  "sort",
  "stat",
  "tail",
  "tar",
  "touch",
  "tr",
  "uname",
  "uniq",
  "wc",
  "xargs",
  "zsh",
]);

// Shell builtins run without a program on the runner's PATH.
const SHELL_BUILTINS: ReadonlySet<string> = new Set([
  ":",
  ".",
  "[",
  "[[",
  "cd",
  "echo",
  "exit",
  "export",
  "false",
  "local",
  "pwd",
  "printf",
  "read",
  "return",
  "set",
  "shift",
  "source",
  "test",
  "trap",
  "true",
  "unset",
  "wait",
]);
// Keywords that open or close a compound command and carry no command word.
const SHELL_HEADERS: ReadonlySet<string> = new Set([
  "case",
  "done",
  "esac",
  "fi",
  "for",
  "in",
  "until",
  "while",
]);
// Keywords that may precede the command word on the same line.
const SHELL_PREFIXES: ReadonlySet<string> = new Set([
  "!",
  "{",
  "}",
  "do",
  "elif",
  "else",
  "if",
  "then",
]);
// Programs that run their argument as the command to map.
const WRAPPERS: ReadonlySet<string> = new Set(["command", "exec", "nohup", "time"]);
const SHELL_PROGRAMS: ReadonlySet<string> = new Set(["bash", "sh", "zsh"]);
const ASSIGNMENT = /^[A-Za-z_][A-Za-z0-9_]*=/;
const MAX_NESTING = 16;

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
  const pattern = /(?:[^\s'"]+|'[^']*'|"[^"]*")+/g;
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

// Where a gate's command can be read from: the effigy task table, and the repo
// files a gate runs as scripts. Without a resolver, only the command line itself
// is mapped.
export type ToolResolver = {
  readonly effigyTask: (name: string) => ReadonlyArray<string>;
  readonly readScript: (path: string) => string;
};

export function toolsNeededByCommand(command: string, resolver?: ToolResolver): Array<string> {
  const trimmed = command.trim();
  if (trimmed.length === 0) {
    throw new Error("cannot map an empty gate command to a runner tool");
  }
  const tools = commandTools(trimmed, resolver, 0);
  if (tools.length === 0) {
    throw new Error(`cannot map gate command to a tool without executing it: ${trimmed}`);
  }
  return tools;
}

// Tools one command line needs: its own program, the cargo plugins its Cargo
// launches name, and whatever a nested body runs (`sh -c`, an effigy task, a
// script it invokes).
function commandTools(
  command: string,
  resolver: ToolResolver | undefined,
  depth: number,
): Array<string> {
  return wordTools(tokenize(command), resolver, depth, command);
}

// Drops redirections (`> file`, `2>&1`, `>&2`); none of them names a program.
function stripRedirections(words: ReadonlyArray<string>): Array<string> {
  const kept: Array<string> = [];
  for (let index = 0; index < words.length; index += 1) {
    const word = words[index] ?? "";
    if (/^\d*(?:&?>>?|<)/.test(word)) {
      if (/^\d*(?:&?>>?|<)&?$/.test(word)) {
        index += 1;
      }
      continue;
    }
    kept.push(word);
  }
  return kept;
}

function wordTools(
  words: ReadonlyArray<string>,
  resolver: ToolResolver | undefined,
  depth: number,
  label: string,
): Array<string> {
  if (depth > MAX_NESTING) {
    throw new Error(`gate command nests too deeply, so it may call itself: ${label}`);
  }
  const argv = commandWords(stripRedirections(words));
  const program = argv[0];
  if (program === undefined || SHELL_HEADERS.has(program) || SHELL_BUILTINS.has(program)) {
    return [];
  }
  if (program.startsWith("$") && !isLauncher(program)) {
    throw new Error(`cannot map gate command to a tool, its program is a variable: ${label}`);
  }

  // `rust-command.sh exec ... -- <tool>` leases one tool that calls Cargo itself.
  // The leased tool is a command of its own; a variable here is the caller's
  // forwarded arguments, so only the Cargo the lease always needs is named.
  const execAt = argv.findIndex((word, index) => isLauncher(word) && argv[index + 1] === "exec");
  if (execAt !== -1) {
    const dash = argv.indexOf("--", execAt + 2);
    const leased = dash === -1 ? [] : argv.slice(dash + 1);
    const leasedTools =
      leased[0] === undefined || leased[0].startsWith("$")
        ? []
        : wordTools(leased, resolver, depth + 1, label);
    return [...new Set(["cargo", ...leasedTools])];
  }

  const nested = nestedTools(argv, resolver, depth);
  if (SHELL_PROGRAMS.has(program) && argv[1] === "-c" && nested.length === 0) {
    throw new Error(`cannot map gate command to a tool without executing it: ${label}`);
  }

  const needs = new Set<string>();
  for (const subcommand of cargoSubcommands(argv)) {
    needs.add("cargo");
    if (!STOCK_CARGO_SUBCOMMANDS.has(subcommand)) {
      needs.add(`cargo-${subcommand}`);
    }
  }
  const launches = program === "cargo" || isLauncher(program);
  if (
    !launches &&
    (!STOCK_RUNNER_COMMANDS.has(program) || (needs.size === 0 && nested.length === 0))
  ) {
    needs.add(program);
  }
  return [...new Set([...needs, ...nested])];
}

function nestedTools(
  argv: ReadonlyArray<string>,
  resolver: ToolResolver | undefined,
  depth: number,
): Array<string> {
  const program = argv[0] ?? "";
  const first = argv[1];
  if (program === "effigy" && resolver !== undefined && first !== undefined) {
    return resolver.effigyTask(first).flatMap((line) => commandTools(line, resolver, depth + 1));
  }
  if (SHELL_PROGRAMS.has(program) && first === "-c") {
    const body = argv[2];
    return body === undefined ? [] : scriptTools(body, resolver, depth + 1);
  }
  if (resolver !== undefined) {
    if (SHELL_PROGRAMS.has(program) && first !== undefined && isScriptPath(first)) {
      return scriptTools(resolver.readScript(first), resolver, depth + 1);
    }
    if (isScriptPath(program)) {
      return scriptTools(resolver.readScript(program), resolver, depth + 1);
    }
  }
  return [];
}

// Each simple command of a shell body. A `NAME=value` word set earlier in the
// body stands for its value in later commands, so `"$runner" clippy` is seen as
// the launcher it names.
function scriptTools(
  script: string,
  resolver: ToolResolver | undefined,
  depth: number,
): Array<string> {
  if (depth > MAX_NESTING) {
    throw new Error(`gate script nests too deeply, so it may call itself: ${script}`);
  }
  const { commands, substitutions, functions } = splitShell(script);
  const defined = new Set(functions);
  const variables = new Map<string, string>();
  const tools: Array<string> = substitutions.flatMap((body) =>
    scriptTools(body, resolver, depth + 1),
  );
  for (const command of commands) {
    const expanded = expandVariables(command, variables);
    const words = tokenize(expanded);
    for (const word of words) {
      if (!ASSIGNMENT.test(word)) {
        break;
      }
      const equals = word.indexOf("=");
      variables.set(word.slice(0, equals), unquote(word.slice(equals + 1)));
    }
    const called = commandWords(words)[0];
    if (called !== undefined && defined.has(called)) {
      // A call to a function this script defines; its body is analysed above.
      continue;
    }
    tools.push(...wordTools(words, resolver, depth + 1, expanded));
  }
  return tools;
}

// Splits shell text into simple commands. Operators and newlines end one; quoted
// text stays whole; heredoc bodies, arithmetic and array values are not commands.
// Each `$(...)` or backtick body is returned separately, so the program inside it
// is still mapped. Function names the text defines are returned too.
export type ShellText = {
  readonly commands: Array<string>;
  readonly substitutions: Array<string>;
  readonly functions: Array<string>;
};

export function splitShell(script: string): ShellText {
  const text = stripHeredocs(script.replaceAll(/\\\r?\n/g, " "));
  const commands: Array<string> = [];
  const substitutions: Array<string> = [];
  const functions: Array<string> = [];
  let current = "";
  let quote: "'" | '"' | undefined;
  const flush = (): void => {
    if (current.trim() !== "") {
      commands.push(current.trim());
    }
    current = "";
  };
  for (let index = 0; index < text.length; index += 1) {
    const ch = text.charAt(index);
    const next = text.charAt(index + 1);
    if (quote === "'") {
      if (ch === "'") {
        quote = undefined;
      }
      current += ch;
      continue;
    }
    if (ch === "$" && next === "(") {
      const close = matchingParen(text, index + 1);
      if (text.charAt(index + 2) === "(") {
        // `$((...))` is arithmetic and runs no command.
        current += "0";
      } else {
        substitutions.push(text.slice(index + 2, close));
        current += "__subst__";
      }
      index = close;
      continue;
    }
    if (ch === "`") {
      const close = text.indexOf("`", index + 1);
      if (close === -1) {
        throw new Error("unterminated backtick in a gate script");
      }
      substitutions.push(text.slice(index + 1, close));
      current += "__subst__";
      index = close;
      continue;
    }
    if (quote === '"') {
      if (ch === '"') {
        quote = undefined;
      }
      current += ch;
      continue;
    }
    if (ch === "[" && next === "[" && startsCommand(current)) {
      // `[[ ... ]]` is one test: its `||` and `&&` are not command separators.
      const close = text.indexOf("]]", index);
      if (close === -1) {
        throw new Error("unterminated [[ in a gate script");
      }
      current += text.slice(index, close + 2);
      index = close + 1;
      continue;
    }
    if (ch === "(" && next === "(") {
      // `((...))` is the arithmetic test a shell `if` runs; it runs no command.
      flush();
      index = matchingParen(text, index);
      continue;
    }
    if (ch === "(" && next === ")") {
      // `name() {` defines a function; its name is not a command.
      functions.push(current.trim());
      current = "";
      index += 1;
      continue;
    }
    if (ch === "(" && current.endsWith("=")) {
      // `name=( ... )` is an array value, not a subshell.
      index = matchingParen(text, index);
      current += "()";
      continue;
    }
    if (ch === "'" || ch === '"') {
      quote = ch;
      current += ch;
      continue;
    }
    if (ch === "\\") {
      current += ch + next;
      index += 1;
      continue;
    }
    if (ch === "#" && (current === "" || /\s$/.test(current))) {
      const eol = text.indexOf("\n", index);
      if (eol === -1) {
        break;
      }
      index = eol - 1;
      continue;
    }
    if (ch === "&" && (next === ">" || current.endsWith(">"))) {
      current += ch;
      continue;
    }
    if (ch === "\n" || ch === ";" || ch === "&" || ch === "|" || ch === "(" || ch === ")") {
      flush();
      continue;
    }
    current += ch;
  }
  flush();
  return { commands, substitutions, functions };
}

// Whether the text so far leaves the next word in command position: nothing
// yet, or only a keyword such as `if` or `then` that precedes a command.
function startsCommand(current: string): boolean {
  const pending = current.trim();
  return pending === "" || /(?:^|\s)(?:if|elif|then|do|else|while|until|!)$/.test(pending);
}

// Removes heredoc bodies: the lines from a `<<WORD` line through its terminator.
// `<<<` is a here-string, not a heredoc.
function stripHeredocs(text: string): string {
  const kept: Array<string> = [];
  let terminator: { readonly word: string; readonly tabs: boolean } | undefined;
  for (const line of text.split("\n")) {
    if (terminator !== undefined) {
      const candidate = terminator.tabs ? line.replace(/^\t+/, "") : line;
      if (candidate === terminator.word) {
        terminator = undefined;
      }
      continue;
    }
    kept.push(line);
    const match = /(?<!<)<<(-?)\s*(["']?)([A-Za-z_][A-Za-z0-9_]*)\2/.exec(line);
    if (match) {
      terminator = { word: match[3] ?? "", tabs: match[1] === "-" };
    }
  }
  return kept.join("\n");
}

function matchingParen(text: string, open: number): number {
  let depth = 0;
  for (let index = open; index < text.length; index += 1) {
    const ch = text.charAt(index);
    if (ch === "(") {
      depth += 1;
    } else if (ch === ")") {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }
  throw new Error("unterminated ( in a gate script");
}

function expandVariables(text: string, variables: ReadonlyMap<string, string>): string {
  return text.replaceAll(
    /\$\{?([A-Za-z_][A-Za-z0-9_]*)\}?/g,
    (match, name: string) => variables.get(name) ?? match,
  );
}

function unquote(value: string): string {
  const first = value.charAt(0);
  if (value.length >= 2 && (first === '"' || first === "'") && value.endsWith(first)) {
    return value.slice(1, -1);
  }
  return value;
}

function isLauncher(word: string): boolean {
  return word.endsWith("rust-command.sh");
}

function isScriptPath(word: string): boolean {
  return word.endsWith(".sh") && !isLauncher(word);
}

// Drops what precedes the command word: assignments, keywords, and the
// wrappers that exec their argument (`nice -n 5 bash ...`, `env X=1 cmd`).
function commandWords(words: ReadonlyArray<string>): Array<string> {
  const skip = leadingSkip(words);
  return skip === 0 ? [...words] : commandWords(words.slice(skip));
}

function leadingSkip(words: ReadonlyArray<string>): number {
  const first = words[0] ?? "";
  if (ASSIGNMENT.test(first) || SHELL_PREFIXES.has(first) || WRAPPERS.has(first)) {
    return 1;
  }
  const isOption = (word: string | undefined): boolean =>
    word !== undefined && (ASSIGNMENT.test(word) || word.startsWith("-"));
  if (first === "env") {
    let skip = 1;
    while (isOption(words[skip])) {
      skip += 1;
    }
    return skip;
  }
  if (first === "nice") {
    if (words[1] === "-n") {
      return 3;
    }
    let skip = 1;
    while (words[skip]?.startsWith("-")) {
      skip += 1;
    }
    return skip;
  }
  return 0;
}

// Resolves effigy task bodies from effigy.toml, read as TOML. A table task's
// `run` lines are its body; a `{ rhai = … }` task cannot be mapped, so it fails.
export function repoToolResolver(root: string): ToolResolver {
  const parsed: unknown = Bun.TOML.parse(readFileSync(join(root, "effigy.toml"), "utf8"));
  const tasks = isMapping(parsed) ? parsed["tasks"] : undefined;
  if (!isMapping(tasks)) {
    throw new Error("effigy.toml has no [tasks] table");
  }
  return {
    effigyTask(name: string): Array<string> {
      const entry = tasks[name];
      if (entry === undefined) {
        throw new Error(`effigy.toml has no task ${name}, which a release gate runs`);
      }
      return taskCommandLines(entry, name);
    },
    readScript(path: string): string {
      return readFileSync(join(root, path), "utf8");
    },
  };
}

function taskCommandLines(entry: unknown, name: string): Array<string> {
  if (typeof entry === "string") {
    return [entry];
  }
  if (Array.isArray(entry)) {
    return entry.flatMap((item: unknown) => taskCommandLines(item, name));
  }
  if (isMapping(entry)) {
    if (typeof entry["task"] === "string") {
      return [`effigy ${entry["task"]}`];
    }
    if ("run" in entry) {
      return taskCommandLines(entry["run"], name);
    }
  }
  throw new Error(`effigy task ${name} has a form this check cannot map: ${JSON.stringify(entry)}`);
}

export function cargoInstallCrates(script: string): Array<string> {
  const crates: Array<string> = [];
  for (const line of commandLines(script)) {
    const argv = leadingCommand(tokenize(line));
    if (argv[0] !== "cargo" || argv[1] !== "install") {
      continue;
    }
    for (let index = 2; index < argv.length;) {
      const token = requireValue(argv[index], "argv[index]");
      if (token.startsWith("-")) {
        const flag = requireValue(token.split("=")[0], 'token.split("=")[0]');
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

function cargoSubcommands(argv: ReadonlyArray<string>): Array<string> {
  const subcommands: Array<string> = [];
  // A gate that launches Cargo through `scripts/rust-command.sh` runs `cargo`
  // on a CI runner and `oxide` on a developer machine, so the word `cargo` may
  // not appear at all. The runner tool is the same either way, so read the
  // subcommand after the launcher exactly as after a literal `cargo`.
  argv.forEach((word, index) => {
    if (word !== "cargo" && !isLauncher(word)) {
      return;
    }
    const subcommand = argv
      .slice(index + 1)
      .find((token) => !token.startsWith("+") && !token.startsWith("-"));
    if (subcommand !== undefined) {
      subcommands.push(subcommand);
    }
  });
  return subcommands;
}

function leadingCommand(argv: Array<string>): Array<string> {
  let index = 0;
  while (
    index < argv.length &&
    /^[A-Za-z_][A-Za-z0-9_]*=/.test(requireValue(argv[index], "argv[index]"))
  ) {
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
    (match, space: string | undefined) => space ?? match,
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
  const resolver = repoToolResolver(root);
  const requirements = gates
    .filter(({ name }) => name !== "workspace")
    .flatMap(({ name, command }) =>
      toolsNeededByCommand(command, resolver).map((tool) => ({
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
