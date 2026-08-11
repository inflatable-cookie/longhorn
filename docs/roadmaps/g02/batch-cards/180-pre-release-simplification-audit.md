# 180 Pre-release Simplification Audit

Status: ready — findings only, no work done
Owner: Tom
Roadmap: g02 planning checkpoint
Governing refs: contract 012
Depends on: Card 179 complete
Blocks: nothing; each finding is independently actionable

## Why

Card 179 removed an abstraction that earned nothing and had complicated every
consumer for a generation. It was found by measuring, not by intuition: one
container per document in every fixture, a binding created once and never
rebound. This audit asks the same question of the rest of the repository before
a first tag makes the answers expensive.

## Measured 2026-08-11

| | |
| --- | --- |
| crates | 42 |
| source lines | 118,598 |
| test lines | 48,087 |
| Tauri adapter crates | 11, totalling 13,433 lines — keep, see Finding 2 |
| crates with zero consumers anywhere | 2 |

## What Is Already Fine

Two things worth stating so nobody spends effort on them.

**Test volume is healthy.** 48,087 test lines against 118,598 source lines is
29%. For a library whose entire value is behavioural guarantees under
expected-revision mutation, that is not bloat. No test should be cut for being
a test.

**There are no compatibility shims left.** `grep` finds "compatibility" in 40
files; every one inspected is a domain concept — restore-archive compatibility,
protocol version negotiation — not a bridge to an older shape. The one genuine
shim, `merge_pre_card179_state`, was deleted on 2026-08-11 once it was
established that no machine holds state worth migrating.

## Finding 1 — Two Crates Have No Consumers

`longhorn-browser` (481 lines) and `longhorn-config-age` (1,059 lines) are
referenced by no `Cargo.toml` and no `use` anywhere in the workspace, the
examples or the prototypes.

Both look deliberate rather than abandoned: browser launch is what contract
019's RFC 8252 flow needs, and the age adapter is the optional encrypted-backup
capability from g01. Neither is dead code by accident.

The question is whether either has a product waiting for it. If not, they are
1,540 lines and two crates carrying maintenance and gate time for a capability
nobody has asked for twice. **Decision needed, not a recommendation.**

## Finding 2 — Eleven Tauri Crates Should Stay Eleven

**Investigated 2026-08-11 and withdrawn.** This was the audit's headline
recommendation. It does not survive its own evidence.

Each Tauri crate depends on its own domain crate and nothing else of substance:
`-notifications` on `longhorn-notifications`, `-history` on `longhorn-history`,
and so on. Merging them means the one crate depends on all twelve domains, so
every domain dependency becomes optional behind a feature.

That is where it stops being a simplification:

- **Feature gating becomes load-bearing for correctness.** Today "does the
  notifications adapter build without `longhorn-history`?" is answered free by
  the dependency graph. Merged, it is answered only by a CI feature matrix that
  does not exist — and `--all-features` cannot answer it, because a missing
  `#[cfg(feature)]` compiles fine when everything is on. The merge would add a
  gate, not remove one.
- **Compile parallelism goes.** Ten of the eleven are mutually independent —
  only `-transfer` depends on `-windowing` — so they build concurrently now.
- **Rebuild blast radius grows.** Touching the notifications adapter rebuilds
  234 lines today and 13,433 after.
- **The stated benefit was wrong.** The claim was that consumers swap eight
  dependency lines for one. They would swap eight dependency lines for one
  dependency line and eight feature flags. Same text, weaker checking.

Checked and clean, so it argues neither way: there are no name collisions.
Every crate already prefixes by domain — `BridgeHostError`, `CommandHostError`,
`HistoryHostError` — and the colliding internal modules (`authority`,
`commands`, `error`, `handler`) would simply nest. The merge is mechanically
easy. That is not the same as worthwhile.

**The count is not the problem.** Forty-two crates looks like over-fracturing
until you see that one thin adapter per domain, each depending only on its
domain, is the shape you would design deliberately. The number is a symptom of
twelve domains, not of a split gone wrong.

## Finding 3 — The `-config` Split Is Half-Retired Already

`longhorn-command-config`, `longhorn-settings-config`, `longhorn-surfaces-config`
and `longhorn-windowing-config` persist their domain's document. Card 179
merged `longhorn-layout-config` into `longhorn-surfaces-config` and nothing was
lost, which is the precedent.

The open question is whether persistence belongs beside its domain rather than
in a sibling crate. Every one of these depends on its domain crate and on
`longhorn-config`, and nothing else depends on them except the applications.
Folding each into its domain would remove four crates and four dependency
edges.

The counter-argument is real and should be tested before acting: a consumer
that wants the domain types without the storage machinery currently gets that
for free. Check whether any consumer actually does.

## Finding 4 — The Inverse Problem

`longhorn-config` is 24,715 lines, five times the next largest crate, and
12,342 of those are `backup`.

This audit is about too many crates, and this is the one place with too few. A
backup and restore subsystem half the size of its host, with its own archive
format, retention policy and inspection model, is a crate that has not been
extracted rather than a module that grew.

Recorded, not recommended. Splitting it is the opposite of the day's direction
and should be taken on its own merits when someone needs it separately.

## Suggested Order

1. Decide on `longhorn-browser` and `longhorn-config-age`. Cheapest, and it
   changes the count the other findings are measured against.
2. Test the `-config` counter-argument, then fold if it does not hold. This is
   now the only structural finding still open.
3. Leave the Tauri crates alone; see Finding 2.
4. Leave `longhorn-config` alone unless something needs `backup` on its own.

## What Not To Do

Do not run a blanket rename or a mechanical crate merge without the compiler
driving it. Card 179 recorded two near-misses from exactly that: a regex meant
to delete two structs took three state primitives with it, and
`PersistedLayoutDocument` was silently renamed because it contains the
substring `LayoutDocument`. Both were caught in under a minute by
`cargo check`, and neither would have been caught by review.
