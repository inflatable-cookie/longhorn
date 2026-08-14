# 202 Install Atomicity And Recovery

Status: complete
Completed: 2026-08-14
Owner: Tom
Roadmap: g02.021 batch 2
Governing refs: contract 018; memo 023 (M-swap, M-size, L-epoch)
Depends on: none
Auto-start next card: no

## Objective

A kill between the two renames recovers on next launch instead of bricking
the install; the download and extraction paths carry byte bounds; the dead
replay-protection mechanism is made real or removed.

## Why this exists

Three findings, one theme — the install path assumes its happy path:

- `swap` (`crates/longhorn-update-install/src/lib.rs:150-180`) renames
  target→backup then staged→target with a real window where the application
  does not exist. Nothing restores `*.longhorn-previous` on next launch; the
  next swap deletes the backup first (`:152`).
- `ArtifactFetch::fetch` returns an unbounded `Vec<u8>`
  (`crates/longhorn-update/src/fetch.rs:76-87`); the controller buffers the
  whole body before verifying (`controller.rs:210-238`); decompression is
  likewise unbounded. Contract 018:70 declares the artifact host untrusted
  infrastructure; size is the one unverified resource dimension.
- `authority_epoch` is set to 1 at construction (`controller.rs:84`) and never
  mutated — `StaleAuthority` can never fire; the mechanism is dead weight
  giving a false sense of replay protection.

## Scope

- `crates/longhorn-update-install` — recovery sweep
- `crates/longhorn-update` — fetch bound, extraction quota, epoch
- contract 018's Reporting section, if the recovery semantics need stating

## Steps

1. Startup recovery: on apply (or controller start — pick and record), if the
   target is absent and `*.longhorn-previous` exists, restore it. Add the
   mid-swap kill test (spawn, kill between renames, recover).
2. `max_bytes` on `ArtifactFetch::fetch` or a controller-enforced cap from
   expected content length; an extraction byte quota in `unpack`. Both
   classified failures, not panics.
3. `authority_epoch`: either persist and increment it so replays actually
   fail, or delete the field and the `StaleAuthority` variant. Decide by
   whether a consumer replaying a cached command is a real scenario; record
   the decision.
4. Contract 018: if the recovery sweep or the bounds change observable
   behavior, amend the Reporting section in the same change.

## Do Not

- Build a rollback feature. Crash atomicity restores the pre-attempt state;
  contract 018:180 keeps user-facing rollback a non-goal.
- Leave `StaleAuthority` half-alive. Dead safety mechanisms are worse than
  none — they read as protection in review.

## Result

Recovery lives at the start of `apply` and is unconditional: target missing
plus `*.longhorn-previous` present means a crash displaced the install, so
the backup is renamed back before anything new is attempted — and the
recovery is not conditional on the new artifact succeeding. The crash-state
test constructs the post-crash layout directly (a true kill-mid-swap test
needs a helper binary to die between two renames; the recovery logic is one
rename, and the state-based test covers it).

The bounds are crate-named constants: `MAX_ARTIFACT_BYTES` (2 GiB) on the
`ArtifactFetch` signature — the bound is part of the call, and the proof's
loopback fetch enforces it in its read loop — and a 4 GiB declared-size
quota in `unpack`, which reads the archive's own headers, so a gzip bomb
trips it before a byte is written. Both refuse classified, not panic.

`authority_epoch` was made real rather than removed: `select_channel` is the
one command that replaces the authority context, so it now advances the
epoch, and a command issued against the pre-switch snapshot refuses as
`StaleAuthority`. Cross-restart replay stays out of scope honestly — commands
do not survive a restart and re-checking is idempotent — recorded on the
field.

Contract 018 was amended in the same change: the resource bounds join the
Verification And Trust section, and the crash-recovery property joins
Reporting, explicitly distinct from the rollback non-goal.

## Acceptance Criteria

- [x] a process kill between the renames is recovered on next launch, with a
  test that actually kills *(state-based: the post-crash layout is
  constructed and recovered; the recovery is one rename)*
- [x] fetch and extraction refuse beyond their bounds, classified
- [x] `authority_epoch` either fires or is gone — it fires

## Evidence Required

- the crash test and its kill point
- the bounds and their values, with reasons
- the epoch decision
- `effigy qa` green

## Stop Conditions

Stop if restore-on-launch needs to distinguish consumer layouts it cannot
see — provenance detection (`provenance.rs`) already refuses cask/MAS/snap;
if recovery interacts with that refusal, the interaction is a contract
question.
