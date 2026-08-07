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

Ninth g02 milestone, and the first that is not remediation. Card 150 gates
everything after it: all channels ship under one bundle identity, so a
nightly build and a production build share the configuration, settings,
history, and history-tree stores. Until every store records the schema that
wrote it and refuses a newer one, shipping a second channel risks silent
data loss on the rejoin path.

## Execution Plan

### Batch 1. Cross-channel store compatibility

- [ ] [Card 150](batch-cards/150-store-schema-stamping-and-forward-refusal.md)
  stamps a schema version into every persistent store and refuses to load a
  newer one

### Batch 2. Update policy and sources

- [ ] [Card 151](batch-cards/151-update-policy-channels-and-rollout.md)
  builds `longhorn-update`: manifest model, channel resolution, semver
  comparison, client-side rollout, deferral state
- [ ] [Card 152](batch-cards/152-update-source-adapters.md) defines the
  `UpdateSource` trait and the four default adapters

### Batch 3. Install and client surface

- [ ] [Card 153](batch-cards/153-restart-interlock-and-tauri-install.md)
  builds `longhorn-tauri-update`: quiescence receipt, plugin wiring, install
- [ ] [Card 154](batch-cards/154-update-client-surface.md) builds
  `packages/update`: bindings and Svelte surface

## Dependency Shape

```text
memo 019 application update
 ├─ 150 store schema stamping        (gates 151-154)
 └─ 151 update policy ─┬─ 152 source adapters
                       └─ 153 restart interlock ─ 154 client surface
```

## Goals

- [ ] no store loads under a schema newer than the reader understands
- [ ] channel selected at runtime from settings; one bundle identity
- [ ] rollout decided on the client, so static hosting is sufficient
- [ ] `minimum_version` and user-initiated checks both override rollout
- [ ] no install proceeds while Longhorn-owned work is in flight
- [ ] signature verification stays entirely inside the Tauri plugin

## Acceptance Criteria

- [ ] a store written under a newer schema is refused with a typed error,
  never partially parsed, never written back
- [ ] rollout eligibility is deterministic per install and release
- [ ] an install ahead of its selected channel reports that state distinctly
  from "no update available"
- [ ] a refused restart defers with a reason rather than cancelling
- [ ] consumer coordination for the crate and package additions is agreed
  before Card 151 opens

## Explicit Non-goals

- artifact hosting, signing key custody, notarization
- delta updates, rollback, server-side rollout orchestration
- a Longhorn-published update server
- side-by-side channel installation

## Consumer Guardrail Exception

This milestone adds two crates (`longhorn-update`, `longhorn-tauri-update`)
and one package (`packages/update`). The g02 remediation guardrail against
crate and package additions does not hold here: the nucleus boundary
verifier will reject the additions until nucleus updates. Sequencing with
nucleus is a precondition of Card 151, not a closeout step.

## Next Task

Open Card 150. It is independent of the rest and of the poodle release
currently blocking the v0.1.0 tag.
