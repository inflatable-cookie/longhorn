# Longhorn 0.2.1 Publication

2026-09-26. Longhorn `0.2.1` is published: three npm packages at `0.2.1`
(`latest`, MIT) and the annotated tag `v0.2.1`. The release carries
agent-answerable Tauri JS `open`/`save` selection (`g02.045`), so Figmatic can
replace its source links with exact pins.

## Frozen identity

- Tag `v0.2.1` (annotated) dereferences to
  `2605853b378d372934da9f84e1504d6644a1dd46`.
- npm integrity, each verified byte-for-byte against the release run's packed
  artifact:
  - `@inflatable-cookie/longhorn@0.2.1` —
    `sha512-ugD+93f9xJ/4qA9/7hN6OCaGDpCIuXS1KsOArspeZwjltIjT5i/NBr75sXwPEQLHdWvaRp5JyEH78Z1HSGvf8Q==`
  - `@inflatable-cookie/longhorn-poodle-svelte@0.2.1` —
    `sha512-do3tlouZgz3D4xDYGPiy1OCEupSmvqgSeuz5/t0v5crO9DXjYIXwvKly2RBmWA58+4ajs/7Gd7eifwxU1alYxA==`
  - `@inflatable-cookie/longhorn-tauri@0.2.1` —
    `sha512-rCUIgbMlKkafX5AXN4SyOuqPEt28re27TFmhbtJMQUmRJl1JJjWkyq7jKACwhKo74qvYbEAr2ijjkUultxJrrA==`
- Both adapters peer on `@inflatable-cookie/longhorn` `0.2.1` exactly;
  `longhorn-poodle-svelte` keeps Poodle `0.4.2`.
- The Rust crates are `publish = false`. Consumers take them by the git tag.

## What shipped

- **Agent-answerable file selection** (`g02.045`, contract 022): a
  pending-selection registry publishes `longhorn://agent-control/selection`;
  `answer_selection` / `reject_selection` settle the waiter. Consumers bind
  with `bindFileSelection`. Human pickers keep the dialog plugin. `save`
  chooses a target; Longhorn never writes it.

No other runtime change. `g02.046` carried the version surface only: the Rust
workspace and internal pins, the root and eight prototype locks, the three npm
versions and adapter peers, the skill stamp, artifact-proof expectations, the
API reference, and the changelog heading.

## Candidate route

`effigy release prepare --yes --check-gates` could not cut `0.2.1`: it syncs the
root `Cargo.lock` but not the eight workspace-excluded prototype locks, so the
`--locked` prototype gate failed and Effigy rolled back cleanly. `g02.046`
prepared the candidate by hand through Queue (PR #35, independent review) and
added `effigy sync:prototype-locks` as the repeatable pre-gate step. The gate
stayed strict.

`effigy release status --check-gates` then passed all seven gates but exited
nonzero because the already-versioned candidate has an empty `[Unreleased]`
section. The seven gate results are the release evidence; see `PAPERCUTS.md`.

## Evidence

- Exact-commit CI `36196066708` — green on `2605853b`.
- `effigy ci:rehearse` — green on `2605853b`.
- Seven `[release.gates]` — all passed on `2605853b`.
- Dry run `36199404494` (ref `main`) — green: full QA, release gates,
  pack-and-verify.
- Release run `36202254640` (ref `v0.2.1`) — green, 35m49s: full QA, release
  gates, tag version check, pack-and-verify, publish. All three packages
  published on the first pass.
- Registry integrity equals the packed artifact for all three packages.
- Figmatic `g01.052` proved the fresh-leaf New project flow against
  source-linked Longhorn before the cut.

## Remaining

- Figmatic repoints from source links to `0.2.1` / `v0.2.1`. Cross-repo and
  consumer-owned.
- HTML file input and Rust-side pickers stay separate, uncommitted follow-ups.
- The `dev` migration forward from `0.2.0` still stands. Remove it at the next
  breaking release.
