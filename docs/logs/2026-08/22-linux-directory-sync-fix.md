# Linux Directory Sync Fix

Date: 2026-08-22
Scope: `longhorn-config` store publication; Soundcheck Linux acceptance
finding (its g04.029 lane)

## What happened

Soundcheck's Linux rendered-window acceptance found that every
`DurabilityRequirement::Durable` publication fails on Linux: the
post-rename directory sync cloned the cap-std `Dir` and called
`sync_all`, but cap-std opens directory capabilities with `O_PATH` on
Linux and `fsync(2)` on an `O_PATH` fd is `EBADF`. `Atomic`
publications silently degraded to `Durability::FileSynced`; `Durable`
ones failed — in Soundcheck, window-placement persistence errored, the
reveal watchdog never saw `placement_ready`, and the protected window
was never revealed. macOS has no `O_PATH`, which is why four
generations of macOS proofs never surfaced it.

## Fix

`sync_directory` in `crates/longhorn-config/src/store/publication.rs`:
on Linux, reopen `.` relative to the capability as a real
`O_RDONLY | O_DIRECTORY | O_CLOEXEC` fd (`rustix::fs::openat`) and
fsync that; off Linux the original clone-and-sync is byte-identical.
`rustix` (already in the graph via cap-std, same resolved version) is a
Linux-target-only direct dependency. Durability semantics and the
`Atomic` degradation lane are unchanged; no API change — Soundcheck
consumes via path-dep and rev pin.

The backup publication path
(`backup/archive/publication/publish.rs`) was checked for the same
pattern and does **not** have the bug: it opens the parent with
`std::fs::File::open` (a real `O_RDONLY` fd, valid for fsync).

## Validation

- New regression test
  `durable_publication_syncs_the_directory_on_this_platform`: a
  `Durable` publication with no injected failure must reach
  `FileAndDirectorySynced` — on Linux this fails as `EBADF` before the
  fix; green on macOS.
- Full `longhorn-config` suite on macOS: 150 tests green; clippy and
  fmt clean. `effigy qa`: every Rust lane green (workspace clippy,
  check, tests, fmt — the board is fail-fast and reached TS); the board
  then failed at `check:ts` inside the *linked sibling Poodle checkout*
  (`node_modules/@inflatable-cookie/poodle-svelte/src/types.ts`
  referencing `AgentSubagentItem`/`TranscriptSubagentGroup`, absent
  from the linked `poodle-core`). That is Poodle-link drift in the
  environment, not this change — the batch is Rust and docs only and
  touches nothing under `packages/`. Operator item: refresh the Poodle
  link (rebuild poodle-core or re-run the link path) and re-run
  `check:ts`.
- Linux verification here is compile-plus-unit-level:
  `cargo check` and clippy for `aarch64-unknown-linux-gnu` (Soundcheck's
  reported architecture) pass; no Linux runtime available in this
  session (Docker daemon down). Soundcheck verifies live by re-running
  its rendered-window acceptance against the fixed revision.
