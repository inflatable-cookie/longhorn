# g02.041 Staged Update Protocol

Owner: Tom
Created: 2026-09-22
Governing refs: contract 018 (`Staged Download And Install`)
Depends on: no other task
UI classification: none

## Outcome

`longhorn-update` retains an identity-bound verified staged artifact between a
prepare step and an apply step, reports byte progress while the transfer runs,
and lets a surface hold the update through a deferred install, cancel it, or
receive a typed failure — without weakening verification or gating.

## Context

Contract 018 was amended 2026-09-22: the single-call install sequence is
replaced by a staged protocol so a consumer (`acowtancy` Desktop contract 038)
can show observable byte progress and offer "Restart to update or Later". The
released `0.1.0` `UpdateController::install` fetches, verifies, gates, and
applies in one locked call; progress is only the last report, and nothing is
retained between calls.

## Ready-State Rubric

- [x] The operator approved the contract 018 amendment (2026-09-22).
- [x] The consumer requirement is settled (Desktop contract 038).
- [x] No other Longhorn lane owns the update protocol.
- [x] UI classification is none.

## Work

1. Protocol types: a `StagedArtifact` binding version, channel, and digest to
   verified bytes; `UpdatePrepareCommand`, `UpdateApplyCommand`, and
   `UpdateCancelCommand`; extend `UpdateProgressProjection` so `Downloading`
   carries a live fraction and `ReadyToInstall` is a retained state.
2. Controller: split `install` into `prepare` (fetch → verify → retain) and
   `apply` (consume the staged artifact), plus cancel/discard. The staged
   artifact holds only `VerifiedArtifact` bytes; a verification failure
   discards and never retains.
3. Live progress: an out-of-band observer/event seam so the fetch reports byte
   progress during the transfer, not only after the call returns.
4. Tauri seam: `prepare`/`apply`/`cancel` host service calls with progress
   emitted out of band, and no authority lock held across the download.
5. Generated projections and the TypeScript controller expose the staged state
   and live progress.
6. Tests: staged retention across a deferred install, cancel, typed errors,
   progress observed mid-transfer, and the type-level "apply takes only
   `VerifiedArtifact`" invariant.

## Acceptance and review oracle

| Invariant | Required proof |
| --- | --- |
| Verification unchanged | `apply` accepts only `VerifiedArtifact`; no staging path accepts unverified bytes |
| Staged retention | a prepared artifact survives a deferred install and applies later |
| Live progress | a byte report is observed during the transfer, before `prepare` returns |
| Cancel | an explicit cancel discards the staged artifact and returns to `Idle` |
| Typed errors | fetch and verification failures are typed on the prepare outcome |

Validation uses the focused `longhorn-update` selectors plus `check:bindings`,
then `effigy qa`. An independent exact-head review must inspect the
staged-artifact type invariant, not source-regex assertions.

## Stop conditions

Stop if the staged handle cannot hold only verified bytes without changing the
`UpdateInstaller` trait, or if any design would weaken the gate. Escalate to
Chatterbox.

## Evidence

On completion, record: the changed crates, the new protocol types, the progress
seam, and the test results.
