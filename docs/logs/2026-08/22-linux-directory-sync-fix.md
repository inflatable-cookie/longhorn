# Linux And Windows Directory Sync Fixes

Date: 2026-08-22
Scope: `longhorn-config` store publication; Soundcheck Linux acceptance
finding (its g04.029 lane) and the Windows sibling (its g04 card 144)

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

## Windows sibling (same day)

Soundcheck's Windows lane found the non-Linux arm equally broken on
Windows: `FlushFileBuffers` on a directory handle fails with
`ERROR_ACCESS_DENIED` — cap-std handles lack
`FILE_FLAG_BACKUP_SEMANTICS`, and the call is not defined for directory
handles even with it. Same downstream shape: every `Durable`
publication failed, startup profile adoption failed, placement never
persisted (Windows 11 ARM64, five failures with the signature).

**Posture chosen: documented no-op** (Soundcheck's option a). Windows
has no directory-flush operation; NTFS journals the rename metadata
itself, and `std`/`tokio`/Soundcheck's own sites take the same stance.
`sync_directory` is now three arms: Linux reopen-and-fsync (unchanged
from `36504692`), Windows documented no-op, everything else the
original clone-and-sync byte-identical. On Windows,
`Durability::FileAndDirectorySynced` means the platform's
directory-durability guarantee applies — there is no stronger operation
to perform. A best-effort backup-semantics flush was rejected as
ceremony: it cannot add durability and its failure handling would blur
the `Durable` contract.

Validation: the existing regression test pins
`Durable → FileAndDirectorySynced` on every platform (on Windows it
exercises the no-op arm; pre-fix it failed as os error 5); macOS suite
150 green, clippy/fmt clean; `cargo check` green for
`aarch64-pc-windows-msvc` (Soundcheck's architecture) and
`aarch64-unknown-linux-gnu` (arm unchanged). Windows verification here
is compile-plus-unit-level; Soundcheck's card 147 rendered acceptance
on its Windows VM is the live proof.

## Centralization (same day, second Windows follow-up)

Soundcheck's VM verification (158/5 → 162/1) proved the publication fix
and exposed the class elsewhere. Repo sweep found four directory-sync
sites total; all now route through one platform-split module,
`crates/longhorn-config/src/dir_sync.rs` (`sync_dir_handle` for cap-std
capabilities, `sync_dir_path` for paths — Linux reopen-and-fsync where a
capability is in hand, Windows documented no-op, macOS/other unixes
byte-identical):

1. `store/publication.rs` — delegated (was the `36504692`/`40797735`
   local three-arm helper).
2. `atomic_file.rs::sync_directory` — carried the storage-transition
   path; `File::open` on a directory is `ERROR_ACCESS_DENIED` on
   Windows, so profile adoption failed at startup. Now delegates.
3. `backup/archive/publication/publish.rs` — the earlier "backup path
   not affected" note was Unix-only reasoning and wrong for Windows:
   `CreateFileW` cannot open a directory at all, so backup publication
   failed at `OpenParent` before ever reaching the sync. The parent is
   now validated by metadata at the same stage and the barrier goes
   through `sync_dir_path`.
4. `backup/restore/live_io.rs::delete_state` — the exact cap-std
   `into_std_file().sync_all()` pattern removed from publication
   (broken on Linux and Windows both). Now `sync_dir_handle`.

Repo-wide sweep confirms no directory-sync pattern exists outside the
module. New module test pins both barrier forms Ok on every platform;
suites 151 green on macOS; `cargo check` green for
`aarch64-pc-windows-msvc` and `aarch64-unknown-linux-gnu`. Windows
verification remains compile-plus-unit here; Soundcheck's VM rerun is
the live proof.
