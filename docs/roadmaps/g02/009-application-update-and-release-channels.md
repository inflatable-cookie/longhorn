# g02.009 Application Update And Release Channels

Status: ready
Owner: Tom
Updated: 2026-08-07
Governing refs: contract 018; contracts 004, 012, and 017; research memo 019
Depends on: none within g02

## Outcome

In-app update for consuming applications: an application notices a new
release on its selected channel, offers it, and installs it without losing
work in flight. Longhorn owns channel policy, client-side rollout, source
adapters, and restart readiness. Hosting, signing, and installation stay
outside the boundary.

## Generation Runway

Ninth g02 milestone, and the first that is not remediation.

Card 150 was compiled as a gate on the belief that no store records the
schema that wrote it. That was wrong — all four stores already stamp and
refuse forward, and the write path refuses to overwrite a store it could not
load. Card 150 is now a proof-and-classification card and gates nothing;
memo 019 carries the correction and the evidence.

## Execution Plan

### Batch 1. Cross-channel store compatibility

- [x] [Card 150](batch-cards/150-store-schema-stamping-and-forward-refusal.md)
  proves the existing forward-refusal end-to-end per store and gives it one
  shared classification

### Batch 2. Update policy and sources

- [x] [Card 151](batch-cards/151-update-policy-channels-and-rollout.md)
  builds `longhorn-update`: manifest model, channel resolution, semver
  comparison, client-side rollout, deferral state
- [x] [Card 152](batch-cards/152-update-source-adapters.md) defines the
  `UpdateSource` trait and the four default adapters

### Batch 3. Install and client surface

- [ ] [Card 153](batch-cards/153-restart-interlock-and-tauri-install.md)
  builds `longhorn-tauri-update`: quiescence receipt, plugin wiring, install
  — mechanism findings recorded and the quiescence contract landed; host
  wiring needs a packaged proof application (tauri#11392)
- [ ] [Card 154](batch-cards/154-update-client-surface.md) builds
  `packages/update`: bindings and Svelte surface

## Dependency Shape

```text
memo 019 application update
 ├─ 150 cross-channel store proof     (independent)
 └─ 151 update policy ─┬─ 152 source adapters
                       └─ 153 restart interlock ─ 154 client surface
```

150 is independent and can run in any order. 154 consumes its shared
classification to explain a channel rejoin.

## Goals

- [x] no store loads under a schema newer than the reader understands, and
  that property is proved rather than assumed
- [x] channel selected at runtime from settings; one bundle identity
- [x] rollout decided on the client, so static hosting is sufficient
- [x] `minimum_version` and user-initiated checks both override rollout
- [ ] no install proceeds while Longhorn-owned work is in flight
- [ ] signature verification stays entirely inside the Tauri plugin
- [ ] two crates and one package added, following existing naming pairs

## Acceptance Criteria

- [ ] a store written under a newer schema is refused with a typed error,
  never partially parsed, never written back
- [ ] rollout eligibility is deterministic per install and release
- [ ] an install ahead of its selected channel reports that state distinctly
  from "no update available"
- [ ] a refused restart defers with a reason rather than cancelling

## Explicit Non-goals

- artifact hosting, signing key custody, notarization
- delta updates, rollback, server-side rollout orchestration
- a Longhorn-published update server
- side-by-side channel installation

## Consumer Guardrail Exception

This milestone adds two crates (`longhorn-update`, `longhorn-tauri-update`)
and one package (`packages/update`). The g02 remediation guardrail against
crate and package additions was scoped to remediation and does not bind new
capability work. Consumers pick the additions up when they adopt the
feature; nothing here blocks on them.

## Next Task

Open Card 150.
