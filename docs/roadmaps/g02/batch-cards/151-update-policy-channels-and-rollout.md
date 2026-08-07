# 151 Update Policy, Channels, And Rollout

Status: ready
Owner: Tom
Roadmap: g02.009 batch 2
Governing refs: contract 018; research memo 019
Depends on: Card 150; consumer coordination on the new crate
Auto-start next card: no

## Objective

Build `longhorn-update`: the manifest model, channel resolution, version
comparison, client-side rollout gate, and deferral state. No Tauri
dependency, no network access, no installation.

## Scope

- channel manifest model, including rollout metadata and `minimum_version`
- channel resolution from persisted settings
- semver comparison including prerelease ordering
- rollout eligibility as a pure function
- deferral state and its persistence shape
- channel and build identity stamped into the diagnostics seam

## Steps

1. Model the channel manifest. Rollout carries a fraction and a seed;
   `minimum_version` is independent of both.
2. Resolve the active channel from settings at runtime. Nothing is baked in
   at build time.
3. Implement version comparison so prerelease ordering places
   `1.3.0-nightly.x` before `1.3.0`. Channel rejoin must fall out of the
   ordering rather than being special-cased.
4. Implement rollout eligibility as a deterministic function of a persisted
   random install identifier and the manifest seed. The identifier is random
   per installation and derived from nothing external — not hardware, not
   user identity.
5. Implement the two overrides: `minimum_version` offers to every install
   below it regardless of rollout; a user-initiated check bypasses rollout.
   Rollout applies only to `production`.
6. Model the ahead-of-channel state as its own outcome, distinct from "no
   update available", so the client surface can explain it.
7. Model deferral: a decision with a reason, not a silent skip.
8. Tests: eligibility determinism across repeated checks, widening a
   rollout never revokes an existing offer, both overrides, prerelease
   ordering, ahead-of-channel detection.

## Acceptance Criteria

- eligibility is stable for a given install and release across checks
- widening a fraction only ever adds installs
- `minimum_version` and manual checks both override rollout
- ahead-of-channel is distinguishable from no-update
- pure crate: no Tauri, no network, no filesystem beyond injected state
- workspace QA passes

## Evidence Required

- determinism and monotonic-widening test receipts
- the install-identifier derivation and its privacy rationale
- diagnostics stamping proof

## Stop Conditions

- consumer coordination on the crate addition is not agreed; the nucleus
  boundary verifier rejects new crates and this card adds one

## Next Task

Cards 152 and 153, which are independent of each other.
