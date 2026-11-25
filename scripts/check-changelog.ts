import { isRecord } from "../packages/longhorn/src/core/record.ts";
import { parseJsonRecord } from "./proof-json.ts";
// Fail when Effigy's release parser refuses CHANGELOG.md.
//
// longhorn#049 added `### Release facts` under a release heading. The docs
// gates passed, and `effigy release status --check-gates` refused the file
// later. This asks the same parser, through `effigy --json release status`,
// so the check cannot disagree with release tooling about what a category is.
//
// An empty `[Unreleased]` is the normal state right after a release. Effigy
// reports it as a blocker and exits nonzero, so the exit code alone cannot
// tell it from a parse error. The JSON can: a parse error has no `details`,
// while the empty section reports `changelog.valid: true`, `unreleased.empty:
// true`, and that one blocker. Released Effigy 0.14.1 and the local build print
// that shape alike (`effigy.release.status.v1`); they differ on which changelog
// text they accept, which is why `EFFIGY_BIN` exists.
//
// The verdict is the parse result, never the release Run. `release status`
// resolves release state by walking up from the working directory, so a staged
// copy under a checkout that carries an Effigy release journal refuses with
// "release Run state is unreadable" while the same `details` report
// `changelog.valid: true` (longhorn#174). A run state outside this repository
// is not a changelog problem: it is named in the note and does not red the
// docs check. Every other blocker is still a failure.

import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { parseWorkspacePackageVersion } from "./longhorn-version.ts";

const defaultRepoRoot = resolve(import.meta.dir, "..");
const EMPTY_UNRELEASED = "unreleased changelog section has no entries";
// Effigy's wording for its own release journal, not for CHANGELOG.md.
const UNREADABLE_RELEASE_RUN_STATE = "release Run state is unreadable";

export type ChangelogVerdict =
  | { outcome: "pass"; note: string }
  | { outcome: "fail"; reason: string };

// The workflows install the released Effigy through `inflatable-cookie/setup-effigy`,
// which can parse CHANGELOG.md differently from a newer local build
// (longhorn#063: 0.14.1 refused an unindented continuation line). `EFFIGY_BIN`
// points this check at one exact binary; unset, it uses `effigy` from PATH.
export function effigyBinary(): string {
  const override = process.env.EFFIGY_BIN?.trim();
  return override === undefined || override === "" ? "effigy" : override;
}

function parseEnvelope(stdout: string) {
  try {
    return parseJsonRecord(stdout);
  } catch {
    return undefined;
  }
}

/**
 * The blockers Effigy listed, or `undefined` when it named none.
 *
 * A refused `release status` with no named blocker cannot justify a pass: the
 * empty `[Unreleased]` gap always arrives as one blocker, so an absent, empty
 * or non-string list means the cause is unknown, not that the changelog parsed.
 */
function blockerList(details: unknown): Array<string> | undefined {
  if (!isRecord(details)) {
    return undefined;
  }
  const blockers = details.blockers;
  if (!Array.isArray(blockers) || blockers.length === 0) {
    return undefined;
  }
  return blockers.every((blocker): blocker is string => typeof blocker === "string")
    ? blockers
    : undefined;
}

/** The parse result for CHANGELOG.md, or `undefined` when Effigy reported none. */
function changelogParse(details: unknown): { valid: boolean } | undefined {
  if (!isRecord(details) || !isRecord(details.changelog)) {
    return undefined;
  }
  return { valid: details.changelog.valid === true };
}

function isReleaseRunStateGap(blocker: string): boolean {
  return blocker.startsWith(UNREADABLE_RELEASE_RUN_STATE);
}

export function judgeReleaseStatus(stdout: string): ChangelogVerdict {
  const envelope = parseEnvelope(stdout);
  if (envelope === undefined) {
    return { outcome: "fail", reason: `effigy release status did not print JSON:\n${stdout}` };
  }
  if (envelope.ok === true) {
    return { outcome: "pass", note: "Effigy's release parser accepts CHANGELOG.md" };
  }
  const error = isRecord(envelope.error) ? envelope.error : undefined;
  const details = error?.details;
  const blockers = blockerList(details);
  if (changelogParse(details)?.valid === true && blockers !== undefined) {
    // Every blocker other than the empty section has to be release Run state;
    // anything else means the parse context, not just the journal, is broken.
    const unrelated = blockers.filter((blocker) => blocker !== EMPTY_UNRELEASED);
    const onlyRunState = unrelated.every(isReleaseRunStateGap);
    if (onlyRunState) {
      const empty =
        isRecord(details) && isRecord(details.unreleased) && details.unreleased.empty === true;
      const note = empty ? "CHANGELOG.md parses; [Unreleased] is empty" : "CHANGELOG.md parses";
      return {
        outcome: "pass",
        note:
          unrelated.length === 0
            ? note
            : `${note}. Effigy also refused release run state outside this changelog: ${unrelated.join("; ")}`,
      };
    }
  }
  const reason =
    typeof error?.message === "string"
      ? error.message
      : "effigy release status failed without a message";
  return {
    outcome: "fail",
    reason:
      blockers === undefined
        ? `${reason} (no blocker named, so the changelog parse cannot be trusted)`
        : `${reason}: ${blockers.join("; ")}`,
  };
}

type ReleaseConfig = {
  "version-file"?: string;
  "version-path"?: string;
  changelog?: string;
  "pre-1-0"?: boolean;
  "tag-format"?: string;
};

async function readReleaseTable(repoRoot: string): Promise<string> {
  let releaseConfig: ReleaseConfig | undefined;
  for (const relativePath of ["config/release.toml", "effigy.toml"]) {
    const content = await readFile(join(repoRoot, relativePath), "utf8").catch(() => null);
    if (content === null) {
      continue;
    }
    const parsed = Bun.TOML.parse(content) as { release?: ReleaseConfig };
    if (parsed.release) {
      releaseConfig = parsed.release;
      break;
    }
  }
  const versionFile = releaseConfig?.["version-file"] ?? "Cargo.toml";
  const versionPath = releaseConfig?.["version-path"] ?? "workspace.package.version";
  const changelog = releaseConfig?.changelog ?? "CHANGELOG.md";
  const pre10 = releaseConfig?.["pre-1-0"] ?? true;
  const tagFormat = releaseConfig?.["tag-format"] ?? "v{version}";
  return [
    "[release]",
    `version-file = ${JSON.stringify(versionFile)}`,
    `version-path = ${JSON.stringify(versionPath)}`,
    `changelog = ${JSON.stringify(changelog)}`,
    `pre-1-0 = ${String(pre10)}`,
    `tag-format = ${JSON.stringify(tagFormat)}`,
    "",
  ].join("\n");
}

export async function checkChangelog(repoRoot = defaultRepoRoot): Promise<ChangelogVerdict> {
  // Stage only CHANGELOG.md, the workspace version, and the [release] table so
  // a checkout's release journal or workspace state never fails the parse check.
  const stage = await mkdtemp(join(tmpdir(), "check-changelog-stage-"));
  try {
    const [changelog, cargoManifest, effigyToml] = await Promise.all([
      readFile(join(repoRoot, "CHANGELOG.md"), "utf8"),
      readFile(join(repoRoot, "Cargo.toml"), "utf8"),
      readReleaseTable(repoRoot),
    ]);
    const version = parseWorkspacePackageVersion(cargoManifest);
    const minimalCargo = [
      "[workspace]",
      "members = []",
      "",
      "[workspace.package]",
      `version = ${JSON.stringify(version)}`,
      "",
    ].join("\n");
    await Promise.all([
      writeFile(join(stage, "CHANGELOG.md"), changelog),
      writeFile(join(stage, "Cargo.toml"), minimalCargo),
      writeFile(join(stage, "effigy.toml"), effigyToml),
    ]);

    const proc = Bun.spawn([effigyBinary(), "--json", "release", "status"], {
      cwd: stage,
      stdout: "pipe",
      stderr: "pipe",
    });
    const stdout = await new Response(proc.stdout).text();
    // The JSON is the verdict. The exit code is nonzero for the empty-section gap too.
    await proc.exited;
    return judgeReleaseStatus(stdout);
  } finally {
    await rm(stage, { recursive: true, force: true });
  }
}

if (import.meta.main) {
  const repoRoot = process.argv[2] ? resolve(process.argv[2]) : defaultRepoRoot;
  const verdict = await checkChangelog(repoRoot);
  if (verdict.outcome === "fail") {
    console.error(`CHANGELOG.md is refused by Effigy's release parser:\n${verdict.reason}`);
    process.exit(1);
  }
  console.log(verdict.note);
}
