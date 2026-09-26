# Release Version Sweep

2026-09-26. [g02.049](../../roadmaps/g02/049-release-version-sweep.md)
merged through [PR #37](https://github.com/inflatable-cookie/longhorn/pull/37)
at `1ca32352` (worker head `4c151135`). Queue task
`efe79590-638f-4061-929a-ee9cd4621c1f` closed at `03b7cadb`. Independent review
passed on the first head.

## What landed

- One version source: artifact proofs, adapter boundary tests, the API
  reference generator, and the skill check read `workspace.package.version`
  through `scripts/longhorn-version.ts`. A test guards `scripts/**/*.ts` and
  the boundary tests against new literals.
- `effigy release:bump -- <X.Y.Z>` (`scripts/release-bump.ts`): workspace
  version and internal pins, three npm versions and adapter peers, skill
  stamp, changelog heading, prototype locks, API reference. Idempotent at the
  same version; refuses a non-increasing one; fails on any third-party lock
  move.
- `release:gates` is `[release.gates]` minus `workspace`, in declaration
  order: private-candidate, advisories, rustdoc, prototypes, floor, source.
  `check:release-gates` (in `qa`) fails on drift. `release.yml` is unchanged
  and now runs the full non-QA gate set after its `effigy qa` step.
- Release runbook rewritten in `scripts/README.md`. The Effigy gaps stay as
  upstream notes in `PAPERCUTS.md`.

## Evidence

- Scratch `0.2.2` bump (worker and reviewer, both discarded): the diff covers
  every surface the `0.2.1` candidate touched except the proofs, which no
  longer carry the version. The reviewer parsed every lock: only `longhorn-*`
  path packages moved, with no registry entries.
- Worker: `check:prototypes` and `effigy qa` green on the bumped scratch tree.
  Reviewer: `effigy qa` and `check:prototypes` green on the PR head; the
  version-sensitive checks green on the bumped tree.
- Negative fixtures: third-party lock move fails; dropped or reordered gate
  fails the alignment check.

## Limits

Reviewer notes, non-blocking, left for the next release owner:

- `bumpRelease().changed` omits lock rewrites, so it can report
  `idempotent: true` while a lock moved. Read the tree diff.
- `bumpPackageManifest` silently skips a manifest whose `"version"` line has a
  different shape; `bumpSkillStamp` throws instead.
- `readWorkspacePackageVersion` and `longhornVersion` are two names for one
  read.
- A failure after the tracked-file writes (lock sync or API regeneration)
  leaves a partly bumped tree. It fails loudly and commits nothing.
- Effigy `release prepare` still cannot see the excluded locks, and
  `status --check-gates` still exits nonzero on an empty `[Unreleased]`.
