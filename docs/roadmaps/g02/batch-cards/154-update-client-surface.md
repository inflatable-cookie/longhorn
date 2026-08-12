# 154 Update Client Surface

Status: blocked — needs Card 190; scope corrected 2026-08-12
Owner: Tom
Roadmap: g02.009 batch 3
Governing refs: contracts 018, 010, and 013; research memo 019
Depends on: Card 153; Card 190
Auto-start next card: no

## Objective

Build the update client surface: generated bindings and the Svelte surface for
update availability, progress, channel selection, and deferral.

## Correction — 2026-08-12

Two things this card assumed, neither of which holds.

**`packages/update` cannot be built.** g02.013 consolidated eighteen
TypeScript packages into three and is complete. The update surface belongs in
`packages/longhorn/src/update/` and `packages/longhorn-poodle-svelte/src/update/`,
where every other domain lives.

**There is nothing to bind.** Step 1 says "generate and check bindings for the
update domain types", which presupposes wire types. `longhorn-update` derives
`ts_rs::TS` on seven enums and nothing else, has no `protocol` module, no
command envelope, no snapshot and no changed event. `UpdateAvailability`,
`UpdateOffer`, `Deferral` and `QuiescenceReceipt` are serde-ready but not
TS-derived, and nothing aggregates them into something a client can read.

Card 190 supplies that. This card resumes after it.

## Scope

- generated bindings for the update domain, in `packages/longhorn/src/update/`
  per contract 012 as g02.013 left it
- available-update presentation with release notes
- download and install progress
- channel selection
- deferral: install now, or on next quit

## Steps

1. Generate and check bindings for the update domain types.
2. Build the available-update surface: version, notes, and the actions.
3. Build progress presentation for download and install.
4. Build channel selection against the persisted setting.
5. **Surface the ahead-of-channel state explicitly.** An install on
   `1.3.0-nightly.x` that selects production sits ahead of production
   `1.2.9` and receives nothing until `1.3.0` ships. That is correct
   behaviour and reads as a broken updater unless the surface says so. This
   is the single most likely support question the feature generates.
6. Surface a refused restart with its reason, not as a failure.
7. Follow contract 013 for adapter lifetime; Svelte and Tauri stay peers.
8. Tests: each state renders distinctly, including ahead-of-channel and
   deferred-with-reason.

## Acceptance Criteria

- ahead-of-channel is visibly distinct from no-update-available
- a deferred restart shows its reason
- bindings check clean against the Rust surface
- peers stay peers; no hidden duplicate runtime
- workspace QA passes

## Evidence Required

- per-state rendering tests
- bindings check receipt

## Stop Conditions

- the surface cannot express ahead-of-channel without leaking channel policy
  into the client

## Next Task

Close g02.009. Consumer adoption follows separately.
