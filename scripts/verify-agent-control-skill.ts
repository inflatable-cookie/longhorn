#!/usr/bin/env bun
// Drift-lock the agent-control skill against the core crate's wire
// vocabulary (CONTROL_TOOL_NAMES) and the workspace version. Also runs
// finder and install fixtures so qa covers Cards 235-236 in one selector.

import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

import {
  formatDiagnostics,
  formatStdout,
  mcpAddLine,
  scanDiscovery,
} from "../skills/agent-control/scripts/find-instance.ts";
import {
  compareVersions,
  installSkill,
  parseInstallTarget,
  parseVersionStamp,
} from "./install-agent-control-skill.ts";

const repoRoot = resolve(import.meta.dir, "..");
const toolsPath = join(repoRoot, "crates", "longhorn-agent-control", "src", "tools.rs");
const cargoPath = join(repoRoot, "Cargo.toml");
const skillPath = join(repoRoot, "skills", "agent-control", "SKILL.md");
const finderPath = join(repoRoot, "skills", "agent-control", "scripts", "find-instance.ts");

export function parseControlToolNames(source: string): string[] {
  const block = source.match(
    /pub const CONTROL_TOOL_NAMES: &\[&str\] = &\[([\s\S]*?)\];/,
  );
  if (!block) {
    throw new Error(`CONTROL_TOOL_NAMES not found in ${toolsPath}`);
  }
  const names = [...block[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]);
  if (names.length === 0) {
    throw new Error("CONTROL_TOOL_NAMES is empty");
  }
  return names;
}

export function parseSkillToolTable(skill: string): string[] {
  const heading = skill.indexOf("\n| Tool |");
  if (heading < 0) {
    throw new Error("skill is missing a parseable `| Tool |` table");
  }
  const table = skill.slice(heading + 1);
  const names: string[] = [];
  for (const line of table.split("\n")) {
    if (!line.startsWith("|")) break;
    if (line.startsWith("| Tool |") || /^\|[\s-:|]+\|$/.test(line)) continue;
    const cell = line.split("|")[1]?.trim() ?? "";
    const name = cell.match(/^`([^`]+)`$/);
    if (!name) {
      throw new Error(`skill tool table row is not a backticked name: ${line}`);
    }
    names.push(name[1]);
  }
  if (names.length === 0) {
    throw new Error("skill tool table has no tool rows");
  }
  return names;
}

export function parseWorkspaceVersion(cargo: string): string {
  const match = cargo.match(/\[workspace\.package\][\s\S]*?^version = "([^"]+)"/m);
  if (!match) {
    throw new Error("workspace version not found in Cargo.toml");
  }
  return match[1];
}

function sameSet(left: string[], right: string[]): { missing: string[]; extra: string[] } {
  const leftSet = new Set(left);
  const rightSet = new Set(right);
  return {
    missing: right.filter((name) => !leftSet.has(name)),
    extra: left.filter((name) => !rightSet.has(name)),
  };
}

export function driftFailures(options: {
  vocabulary: string[];
  skillTools: string[];
  skillVersion: string | null;
  workspaceVersion: string;
}): string[] {
  const failures: string[] = [];
  const { missing, extra } = sameSet(options.skillTools, options.vocabulary);
  if (missing.length > 0) {
    failures.push(`skill is missing tools: ${missing.join(", ")}`);
  }
  if (extra.length > 0) {
    failures.push(`skill documents tools the server does not expose: ${extra.join(", ")}`);
  }
  if (!options.skillVersion) {
    failures.push("skill is missing a longhorn_version stamp");
  } else if (options.skillVersion !== options.workspaceVersion) {
    failures.push(
      `skill longhorn_version ${options.skillVersion} does not match workspace ${options.workspaceVersion}`,
    );
  }
  return failures;
}

async function checkLiveSkill(): Promise<void> {
  const [tools, cargo, skill] = await Promise.all([
    readFile(toolsPath, "utf8"),
    readFile(cargoPath, "utf8"),
    readFile(skillPath, "utf8"),
  ]);
  const failures = driftFailures({
    vocabulary: parseControlToolNames(tools),
    skillTools: parseSkillToolTable(skill),
    skillVersion: parseVersionStamp(skill),
    workspaceVersion: parseWorkspaceVersion(cargo),
  });
  if (failures.length > 0) {
    throw new Error(failures.join("\n"));
  }
}

function withToolTable(skill: string, tools: string[]): string {
  const heading = skill.indexOf("\n| Tool |");
  if (heading < 0) throw new Error("cannot splice tool table");
  const start = heading + 1;
  let end = start;
  const lines = skill.slice(start).split("\n");
  let consumed = 0;
  for (const line of lines) {
    if (!line.startsWith("|")) break;
    consumed += line.length + 1;
  }
  end = start + consumed;
  const header = "| Tool | Arguments | Result | Limits |\n| --- | --- | --- | --- |\n";
  const rows = tools.map((name) => `| \`${name}\` | args | result | limits |`).join("\n");
  return `${skill.slice(0, start)}${header}${rows}\n${skill.slice(end)}`;
}

function withVersion(skill: string, version: string): string {
  if (!/^longhorn_version:/m.test(skill)) {
    throw new Error("skill is missing a longhorn_version stamp");
  }
  return skill.replace(/^longhorn_version:\s*.*$/m, `longhorn_version: "${version}"`);
}

async function runDriftFixtures(): Promise<void> {
  const [tools, cargo, skill] = await Promise.all([
    readFile(toolsPath, "utf8"),
    readFile(cargoPath, "utf8"),
    readFile(skillPath, "utf8"),
  ]);
  const vocabulary = parseControlToolNames(tools);
  const workspaceVersion = parseWorkspaceVersion(cargo);

  const cases: Array<{ name: string; mutate: (text: string) => string; expect: string }> = [
    {
      name: "removed tool",
      mutate: (text) => withToolTable(text, vocabulary.filter((name) => name !== "snapshot")),
      expect: "missing tools: snapshot",
    },
    {
      name: "added tool",
      mutate: (text) => withToolTable(text, [...vocabulary, "explode"]),
      expect: "does not expose: explode",
    },
    {
      name: "edited tool name",
      mutate: (text) =>
        withToolTable(
          text,
          vocabulary.map((name) => (name === "type" ? "typing" : name)),
        ),
      expect: "typing",
    },
    {
      name: "stale version stamp",
      mutate: (text) => withVersion(text, "0.0.0"),
      expect: "longhorn_version 0.0.0",
    },
  ];

  for (const fixture of cases) {
    const mutated = fixture.mutate(skill);
    const failures = driftFailures({
      vocabulary,
      skillTools: parseSkillToolTable(mutated),
      skillVersion: parseVersionStamp(mutated),
      workspaceVersion,
    });
    const joined = failures.join("\n");
    if (!joined.includes(fixture.expect)) {
      throw new Error(
        `drift fixture "${fixture.name}" expected ${JSON.stringify(fixture.expect)}, got ${JSON.stringify(joined)}`,
      );
    }
  }
}

function tokenFor(pid: number): string {
  const raw = `tok${pid}`.padEnd(43, "A");
  return raw.slice(0, 43);
}

async function writeDiscovery(
  dir: string,
  appId: string,
  pid: number,
  port: number,
  extras: Partial<DiscoveryFields> = {},
): Promise<string> {
  const file = {
    schemaVersion: 1,
    appId,
    pid,
    port,
    token: tokenFor(pid),
    ...extras,
  };
  const path = join(dir, `${appId}-${pid}.json`);
  await writeFile(path, `${JSON.stringify(file, null, 2)}\n`);
  return path;
}

type DiscoveryFields = {
  schemaVersion: number;
  appId: string;
  pid: number;
  port: number;
  token: string;
};

async function runFinder(args: string[]): Promise<{ code: number; stdout: string; stderr: string }> {
  const subprocess = Bun.spawn(["bun", finderPath, ...args], {
    cwd: repoRoot,
    stdout: "pipe",
    stderr: "pipe",
  });
  const [code, stdout, stderr] = await Promise.all([
    subprocess.exited,
    new Response(subprocess.stdout).text(),
    new Response(subprocess.stderr).text(),
  ]);
  return { code, stdout, stderr };
}

async function runFinderFixtures(): Promise<void> {
  const dir = await mkdtemp(join(tmpdir(), "agent-control-finder-"));
  try {
    const empty = await runFinder(["--discovery-dir", dir]);
    if (empty.code === 0) {
      throw new Error("empty discovery dir should exit nonzero");
    }
    if (!empty.stderr.includes("no live agent-control instance")) {
      throw new Error(`empty dir message missing: ${empty.stderr}`);
    }
    if (empty.stderr.includes(tokenFor(1)) || empty.stdout.includes("Bearer")) {
      throw new Error("empty dir leaked a token");
    }

    const livePid = process.pid;
    const stalePid = 999_999_991;
    await writeDiscovery(dir, "dev.example.live", livePid, 49152);
    await writeDiscovery(dir, "dev.example.stale", stalePid, 49153);
    await writeFile(join(dir, "notes.txt"), "ignore me\n");
    await writeFile(join(dir, "broken.json"), "{not json\n");

    const found = await runFinder(["--discovery-dir", dir]);
    if (found.code !== 0) {
      throw new Error(`live instance should exit 0: ${found.stderr}`);
    }
    if (!found.stdout.includes("http://127.0.0.1:49152/mcp")) {
      throw new Error(`finder stdout missing URL: ${found.stdout}`);
    }
    if (!found.stdout.includes(`Authorization: Bearer ${tokenFor(livePid)}`)) {
      throw new Error("finder paste line missing token");
    }
    if (found.stderr.includes(tokenFor(livePid)) || found.stderr.includes(tokenFor(stalePid))) {
      throw new Error(`diagnostics leaked a token:\n${found.stderr}`);
    }
    if (!found.stderr.includes("found 1 live instance")) {
      throw new Error(`diagnostics missing live count: ${found.stderr}`);
    }
    if (!found.stderr.includes("skipped 1 stale file")) {
      throw new Error(`diagnostics missing stale skip: ${found.stderr}`);
    }

    const filtered = await runFinder([
      "--discovery-dir",
      dir,
      "--app-id",
      "dev.example.missing",
    ]);
    if (filtered.code === 0) {
      throw new Error("missing app id should exit nonzero");
    }
    if (!filtered.stderr.includes("no live agent-control instance for app id")) {
      throw new Error(`filtered message missing: ${filtered.stderr}`);
    }

    const scan = await scanDiscovery(dir, undefined, (pid) => pid === livePid);
    const stdout = formatStdout(scan.live);
    const diagnostics = formatDiagnostics(scan, undefined);
    if (!mcpAddLine(scan.live[0].file).includes("claude mcp add --transport http")) {
      throw new Error("paste line is not a claude mcp add command");
    }
    if (diagnostics.includes(tokenFor(livePid))) {
      throw new Error("formatDiagnostics leaked a token");
    }
    if (!stdout.includes(tokenFor(livePid))) {
      throw new Error("paste line should carry the token");
    }
  } finally {
    await rm(dir, { recursive: true, force: true });
  }
}

async function gitInit(dir: string): Promise<void> {
  const subprocess = Bun.spawn(["git", "init", "-q"], {
    cwd: dir,
    stdout: "pipe",
    stderr: "pipe",
  });
  const code = await subprocess.exited;
  if (code !== 0) {
    throw new Error(`git init failed in ${dir}`);
  }
}

async function listRelative(root: string): Promise<string[]> {
  const glob = new Bun.Glob("**/*");
  const paths: string[] = [];
  for await (const path of glob.scan({ cwd: root, dot: true })) {
    if (path === ".git" || path.startsWith(".git/")) continue;
    paths.push(path);
  }
  paths.sort();
  return paths;
}

async function runInstallFixtures(): Promise<void> {
  const scratch = await mkdtemp(join(tmpdir(), "agent-control-install-"));
  const target = join(scratch, "consumer");
  const decoy = join(scratch, "outside.txt");
  await mkdir(target, { recursive: true });
  await writeFile(decoy, "leave me\n");
  try {
    try {
      await installSkill(target);
      throw new Error("non-repo target should be refused");
    } catch (error) {
      if (!(error as Error).message.includes("not a git repository")) {
        throw error;
      }
    }

    await gitInit(target);
    const first = await installSkill(target);
    if (!first.startsWith("installed ")) {
      throw new Error(`first install message: ${first}`);
    }
    const skill = await readFile(
      join(target, ".claude", "skills", "agent-control", "SKILL.md"),
      "utf8",
    );
    if (!parseVersionStamp(skill)) {
      throw new Error("installed skill missing version stamp");
    }
    const finder = await readFile(
      join(target, ".claude", "skills", "agent-control", "scripts", "find-instance.ts"),
      "utf8",
    );
    if (!finder.includes("claude mcp add")) {
      throw new Error("installed finder script missing");
    }

    const second = await installSkill(target);
    if (!second.includes("no-op")) {
      throw new Error(`re-run should be a no-op: ${second}`);
    }

    const touched = await listRelative(target);
    const unexpected = touched.filter(
      (path) => !path.startsWith(".claude/skills/agent-control/"),
    );
    if (unexpected.length > 0) {
      throw new Error(`install touched paths outside the skill dir: ${unexpected.join(", ")}`);
    }
    const decoyText = await readFile(decoy, "utf8");
    if (decoyText !== "leave me\n") {
      throw new Error("install mutated a file outside the target repo");
    }

    if (parseInstallTarget(["--", "--repo", target]) !== target) {
      throw new Error("parseInstallTarget should accept --repo");
    }

    const older = join(scratch, "older-consumer");
    await mkdir(older, { recursive: true });
    await gitInit(older);
    await installSkill(older);
    const olderSkillPath = join(older, ".claude", "skills", "agent-control", "SKILL.md");
    const current = await readFile(olderSkillPath, "utf8");
    await writeFile(
      olderSkillPath,
      current.replace(/^longhorn_version:\s*.*$/m, 'longhorn_version: "0.0.1"'),
    );
    const upgraded = await installSkill(older);
    if (!upgraded.includes("upgraded")) {
      throw new Error(`older stamp should upgrade: ${upgraded}`);
    }

    const newer = join(scratch, "newer-consumer");
    await mkdir(newer, { recursive: true });
    await gitInit(newer);
    await installSkill(newer);
    const newerSkillPath = join(newer, ".claude", "skills", "agent-control", "SKILL.md");
    const newerCurrent = await readFile(newerSkillPath, "utf8");
    await writeFile(
      newerSkillPath,
      newerCurrent.replace(/^longhorn_version:\s*.*$/m, 'longhorn_version: "9.9.9"'),
    );
    try {
      await installSkill(newer);
      throw new Error("newer stamp should refuse overwrite");
    } catch (error) {
      if (!(error as Error).message.includes("refusing to overwrite")) {
        throw error;
      }
    }

    if (compareVersions("0.1.0", "0.1.0") !== 0 || compareVersions("0.0.1", "0.1.0") >= 0) {
      throw new Error("compareVersions is wrong");
    }
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
}

async function runEffigyInstallFixture(): Promise<void> {
  const scratch = await mkdtemp(join(tmpdir(), "agent-control-effigy-install-"));
  const target = join(scratch, "consumer");
  await mkdir(target, { recursive: true });
  await gitInit(target);
  try {
    const subprocess = Bun.spawn(
      ["effigy", "agent-control:install-skill", "--", target],
      {
        cwd: repoRoot,
        stdout: "pipe",
        stderr: "pipe",
      },
    );
    const [code, stdout, stderr] = await Promise.all([
      subprocess.exited,
      new Response(subprocess.stdout).text(),
      new Response(subprocess.stderr).text(),
    ]);
    if (code !== 0) {
      throw new Error(`effigy install failed: ${stdout}\n${stderr}`);
    }
    if (!stdout.includes("installed ")) {
      throw new Error(`effigy install message missing: ${stdout}`);
    }
    await readFile(join(target, ".claude", "skills", "agent-control", "SKILL.md"), "utf8");

    const again = Bun.spawn(["effigy", "agent-control:install-skill", "--", target], {
      cwd: repoRoot,
      stdout: "pipe",
      stderr: "pipe",
    });
    const [againCode, againStdout, againStderr] = await Promise.all([
      again.exited,
      new Response(again.stdout).text(),
      new Response(again.stderr).text(),
    ]);
    if (againCode !== 0) {
      throw new Error(`effigy re-run failed: ${againStdout}\n${againStderr}`);
    }
    if (!againStdout.includes("no-op")) {
      throw new Error(`effigy re-run should be a no-op: ${againStdout}`);
    }
  } finally {
    await rm(scratch, { recursive: true, force: true });
  }
}

const driftOnly = process.argv.includes("--drift-only");

await checkLiveSkill();
await runDriftFixtures();
if (!driftOnly) {
  await runFinderFixtures();
  await runInstallFixtures();
  await runEffigyInstallFixture();
}

console.log(
  JSON.stringify(
    {
      schema: "longhorn.agent-control-skill.v1",
      outcome: "pass",
      driftOnly,
    },
    null,
    2,
  ),
);
