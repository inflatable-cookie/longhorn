# g02.049 Release Version Sweep

Owner: Tom
Created: 2026-09-26
State: ready for Queue delivery
Governing refs: contract 012, `config/release.toml`, `PAPERCUTS.md` release
entries, `docs/logs/2026-09/26-longhorn-0-2-1-publication.md`
Depends on: g02.046 (complete)
UI classification: none

## Outcome

The next Longhorn version bump is one repo-owned command and a reviewable
diff, not a hand sweep over 35 files. `effigy release:gates` means what its
name says for the runner that calls it.

## Context and confirmed decisions

Operator direction 2026-09-26: Longhorn-only. Do not edit Effigy; the Effigy
gaps (no pre-gate hook for excluded locks, `status --check-gates` exiting
nonzero on an already-versioned candidate) stay as upstream notes.

The `0.2.1` candidate (PR #35, `cdd26dbf`) touched: root `Cargo.toml`
workspace version and 49 internal pins, root and eight prototype locks, three
npm package versions and two adapter peers, two boundary tests, thirteen
artifact-proof sites under `scripts/`, the skill stamp, the API reference, and
the changelog heading. `effigy sync:prototype-locks` already rewrites path
versions in locks surgically. `.github/workflows/release.yml` runs
`effigy qa` and then `effigy release:gates`, which is currently
`[release:floor, release:source-consumer, check:prototypes]` — three of the
six declared non-`workspace` gates.

## Work

1. Make the artifact proofs, boundary tests, and any other active version
   expectations read the version from one source (root
   `workspace.package.version` or `packages/longhorn/package.json`, chosen
   once) instead of literals. Historical evidence files keep their literals.
2. Add `effigy release:bump` (Bun script) taking a semver. It updates the
   workspace version and internal pins, the three npm versions and adapter
   peers, the skill stamp, promotes `[Unreleased]` to the new dated heading,
   runs `sync:prototype-locks`, regenerates the API reference, and fails if
   any lock line other than a Longhorn path-package version moved. It is
   idempotent at the same version and refuses a non-increasing version.
3. Make `release:gates` run every `[release.gates]` entry except
   `workspace` (the runner runs `effigy qa` separately), in declaration
   order. Add a check that the two lists stay aligned. Do not edit
   `.github/workflows/`.
4. Rewrite the release runbook in `scripts/README.md` (and the contract 012
   release section if it names steps) as: `release:bump` → PR → exact-commit
   CI → `ci:rehearse` → `release status --check-gates` (read the gate lines;
   the nonzero exit on an empty `[Unreleased]` is known) → dry run → tag →
   publish. Update the PAPERCUTS entries this closes to `[x]`, and add the
   two Effigy gaps as upstream notes.

## Acceptance and review oracle

| Invariant | Required proof |
| --- | --- |
| One-command bump | in a scratch worktree, `release:bump 0.2.2` produces a diff covering every surface the `0.2.1` candidate touched; `effigy qa` and `check:prototypes` pass on it; the scratch result is discarded, not committed |
| Lock safety | the bump's lock diff moves only Longhorn path-package versions; a fixture with a moved third-party line fails |
| No literals left | a search for the current version outside changelog, logs, and evidence returns only the declared single source(s) |
| Gates aligned | `release:gates` equals `[release.gates]` minus `workspace`; the alignment check fails on a deliberate mismatch |
| Nothing released | no version change, tag, or publish lands in this PR |

Run the focused script tests, `effigy check:prototypes`, then `effigy qa`.

## Dispatch manifest

- **Capability:** Bun/TypeScript automation over Cargo and npm manifests.
- **Owned paths:** `scripts/**` (proof version reads, new bump script, sync
  helper), `effigy.toml` release selectors, `config/release.toml` comments,
  `packages/*/tests/boundary.test.ts`, `scripts/README.md`, the contract 012
  release-step text if present, `PAPERCUTS.md`.
- **Reserved closeout:** this card, `docs/roadmaps/g02/README.md`,
  `docs/roadmaps/generation-index.md`, and the delivery log.
- **Concurrency:** parallel with g02.047 and g02.048. Where it overlaps with
  them in `scripts/`, the later PR rebases.
- **Completion:** all oracle rows pass and the runbook matches the tools.
- **Escalation:** Chatterbox if the version source must be a new file, if a
  gate would need weakening, or if the workflow must change.

## Stop conditions

Stop if the bump cannot keep third-party locks stable, if a fix needs an
Effigy or workflow edit, or if the gate alignment would drop a declared gate.
