# 154 Update Client Surface

Status: in progress — Longhorn side complete 2026-08-13; the Svelte
surface remains and is rescoped to Poodle
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

1. [x] Generate and check bindings for the update domain types. Landed with
   Card 190; this card added the validation, the checked port and the client.
2. [ ] Build the available-update surface: version, notes, and the actions.
3. [x] Build progress presentation for download and install. The *behaviour*:
   the controller exposes progress and keeps the absent fraction absent. The
   rendering is step 2's.
4. [ ] Build channel selection against the persisted setting. The controller
   has `selectChannel`; binding it to the persisted setting is the surface's.
5. [x] **Surface the ahead-of-channel state explicitly.** An install on
   `1.3.0-nightly.x` that selects production sits ahead of production
   `1.2.9` and receives nothing until `1.3.0` ships. That is correct
   behaviour and reads as a broken updater unless the surface says so. This
   is the single most likely support question the feature generates.
6. [x] Surface a refused restart with its reason, not as a failure.
7. [x] Follow contract 013 for adapter lifetime; Svelte and Tauri stay peers.
8. [ ] Tests: each state *renders* distinctly, including ahead-of-channel and
   deferred-with-reason. Each state is *readable* distinctly and tested; the
   rendering half waits on step 2.

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

## Outcome, Longhorn side — 2026-08-13

`packages/longhorn/src/update/` has what every other domain has: validation, a
raw port, a checked port, a client, direct and serialized adapters, and a
controller. Twenty-two tests.

The port Card 190 step 4 left returning `unknown` is narrowed. `UpdateClient`
validates outbound commands before they are sent and inbound replies before
they are returned, and the Tauri port now implements the shared `UpdatePort`
rather than a local interface of its own.

**The controller keeps three outcomes apart, and collapsing any two produces a
wrong message.** Committed: state moved. Committed with a deferral: the install
did not happen and that is not a failure — the gate refused because the user
has work in flight, and `deferral` says which. Rejected: the authority refused,
and this is the only one of the three that is a fault. Transport failure is the
fourth and lands in `status`. Step 6 asked for the second of those; the
distinction from the third is what makes it worth anything.

`aheadOfChannel` is its own read rather than a variant a surface has to
remember to switch on. Step 5 calls it the single most likely support question
the feature generates, and a surface that omits the case renders it as
"up to date" by default. A test asserts the two are not the same read.

A command before the first snapshot fails saying so, rather than inventing an
authority epoch the authority would refuse as stale — which would read as a
protocol problem rather than as "nothing has been read yet".

### What remains, and where it belongs

Steps 2 and 4, and the rendering half of step 8. Those are components, and the
operator decision of 2026-08-12 was that general-purpose components do not
belong in Longhorn — Poodle renders, Longhorn supplies behaviour. So the Svelte
surface is Poodle's, built against these reads.

The controller is the contract between them: `availability`, `progress`,
`channel`, `deferral`, `lastRejection` and `aheadOfChannel` are what a surface
needs and all it needs. Poodle cannot depend on Longhorn, so it mirrors the
generated types structurally — which is why the field maps are exported from
this root, so whoever sees both can assert against them and fail on drift.
