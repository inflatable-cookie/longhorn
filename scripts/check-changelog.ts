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
// true`, and that one blocker.

import { resolve } from "node:path";

const defaultRepoRoot = resolve(import.meta.dir, "..");
const EMPTY_UNRELEASED = "unreleased changelog section has no entries";

type ReleaseStatusDetails = {
  blockers?: string[];
  changelog?: { valid?: boolean };
  unreleased?: { empty?: boolean };
};

type ReleaseStatusEnvelope = {
  ok?: boolean;
  error?: { message?: string; details?: ReleaseStatusDetails | null } | null;
};

export type ChangelogVerdict =
  | { outcome: "pass"; note: string }
  | { outcome: "fail"; reason: string };

function parseEnvelope(stdout: string): ReleaseStatusEnvelope | undefined {
  try {
    return JSON.parse(stdout) as ReleaseStatusEnvelope;
  } catch {
    return undefined;
  }
}

function isEmptyUnreleasedGap(details: ReleaseStatusDetails | null | undefined): boolean {
  return details?.changelog?.valid === true
    && details.unreleased?.empty === true
    && Array.isArray(details.blockers)
    && details.blockers.length > 0
    && details.blockers.every((blocker) => blocker === EMPTY_UNRELEASED);
}

export function judgeReleaseStatus(stdout: string): ChangelogVerdict {
  const envelope = parseEnvelope(stdout);
  if (envelope === undefined) {
    return { outcome: "fail", reason: `effigy release status did not print JSON:\n${stdout}` };
  }
  if (envelope.ok === true) {
    return { outcome: "pass", note: "Effigy's release parser accepts CHANGELOG.md" };
  }
  if (isEmptyUnreleasedGap(envelope.error?.details)) {
    return { outcome: "pass", note: "CHANGELOG.md parses; [Unreleased] is empty" };
  }
  return {
    outcome: "fail",
    reason: envelope.error?.message ?? "effigy release status failed without a message",
  };
}

export async function checkChangelog(repoRoot = defaultRepoRoot): Promise<ChangelogVerdict> {
  const proc = Bun.spawn(["effigy", "--json", "release", "status"], {
    cwd: repoRoot,
    stdout: "pipe",
    stderr: "pipe",
  });
  const stdout = await new Response(proc.stdout).text();
  // The JSON is the verdict. The exit code is nonzero for the empty-section gap too.
  await proc.exited;
  return judgeReleaseStatus(stdout);
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
