# g02.046 Agent Selection 0.2.1 Release Candidate

Owner: Tom
Created: 2026-09-25
State: complete — PR #35; published as `0.2.1` 2026-09-26
Governing refs: contract 012, g02.045, PAPERCUTS.md release-version entries
Depends on: g02.045 complete; Figmatic g01.052 source-linked acceptance proved
UI classification: none

## Outcome

A reviewed, merged Longhorn `0.2.1` candidate carries g02.045 with one
coordinated Rust/npm version and clean locked prototype builds. Tagging and npm
publication remain the operator-authorized release step after exact-commit CI,
all seven release gates, and a dry-run publish.

## Why this batch exists

Effigy `release prepare --yes --check-gates` failed on 2026-09-25 after its
temporary `0.2.1` bump: it syncs the root `Cargo.lock`, but the eight excluded
`prototypes/*/Cargo.lock` files still name Longhorn crates at `0.2.0`.
`check:prototypes --locked` therefore fails. Effigy rolled the changes back;
`main` remains clean at `d159d272`. The gap was already recorded during the
`0.2.0` release in `PAPERCUTS.md`. Do not bypass or weaken the locked check.

## Work

1. Prepare the `0.2.1` version batch in a Queue worker branch. Coordinate the
   root Cargo workspace version and internal pins, root and all eight prototype
   locks, three npm package versions and adapter peers, skill version stamp,
   active artifact-proof version expectations, generated API reference, and
   changelog heading. Preserve third-party lock entries and historical release
   records. Follow the proven `0.2.0` manual candidate route because the
   current Effigy prepare command cannot sync excluded locks.
2. Record a repeatable, narrow version-sync procedure in the release-facing
   docs or script so the next candidate can refresh excluded locks before
   `--locked` checks without mutating a gate. Do not edit Effigy or another
   repository.
3. Validate focused version/lock assertions, `effigy check:prototypes`,
   `effigy qa`, and `git diff --check`. An independent reviewer must inspect
   the lock diff, package peers, artifact expectations, and publish boundary.
   Queue merges only a green candidate PR.
4. After merge, the release owner runs exact-commit hosted CI, all seven
   `[release.gates]`, `effigy ci:rehearse`, and the tag-bound npm dry run before
   the authorised tag and publish. The worker does not tag, publish, dispatch
   workflows, or write Figmatic.

## Acceptance

- Every publishable Longhorn Rust crate and npm package reports `0.2.1`;
  adapter peers name the same core version. The skill and active artifact
  proofs agree.
- All eight excluded prototype locks pass `--locked` after the candidate
  bump. Lock diff changes only Longhorn-owned package versions unless a
  separate dependency reason is recorded and approved.
- `0.2.1` changelog describes agent-answerable Tauri JS selection; docs and
  generated API reference agree with the manifests.
- No tag, registry write, GitHub release, workflow edit, or consumer repo write
  occurs in the Queue task. The candidate's exact merge SHA is returned for
  the release gate and publication step.

## Stop conditions

Stop for a third-party lock move, a required runtime/code/API change, any
release gate defect beyond the known excluded-lock sync, or a version mismatch
that cannot be resolved inside this release batch. Return the evidence to
Longhorn Chatterbox; do not skip gates or release from the worker branch.
