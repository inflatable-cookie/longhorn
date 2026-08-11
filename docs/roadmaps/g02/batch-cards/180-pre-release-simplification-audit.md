# 180 Pre-release Simplification Audit

Status: complete — 2026-08-11; every finding closed as keep-as-is
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

## Finding 3 — The `-config` Split Is Load-bearing

**Tested 2026-08-11. Keep them.**

The proposal was to fold `longhorn-command-config`, `longhorn-settings-config`,
`longhorn-surfaces-config` and `longhorn-windowing-config` into their domain
crates, on the precedent of Card 179 folding `longhorn-layout-config` away
without loss.

The counter-argument was that a consumer might want the domain types without
the storage machinery. It is not hypothetical: **twenty sites take a domain
crate without its `-config` sibling.** Eight for windowing, five for settings,
four for surfaces, three for command — including `loophole-actions`, a real
consumer crate that takes `longhorn-command` and persists nothing, and
`longhorn-gpui-windowing`, `longhorn-surface-windowing` and
`longhorn-tauri-windowing`, which reference `longhorn-config` zero times.

Folding would put `longhorn-config` — 24,715 lines, the largest crate here,
containing a 12,342-line backup and restore subsystem — behind all of them. A
GPUI windowing adapter would compile an archive format, a retention policy and
a restore inspection model in order to draw a window.

**Why Card 179's precedent does not transfer.** The container was an
indirection with one implementation and a 1:1 binding that never varied:
removing it deleted a concept. The `-config` split is a dependency boundary,
and it exists so that using a domain does not cost the storage stack. The two
look alike in a crate listing and are opposites in a dependency graph. That
distinction is the useful output of this audit.

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

Nothing. Every structural finding was investigated and every one closed as
keep-as-is:

- `longhorn-browser` and `longhorn-config-age` stay; both have a product coming.
- The eleven Tauri crates stay; see Finding 2.
- The four `-config` crates stay; see Finding 3.
- `longhorn-config` stays whole unless something needs `backup` on its own.

**The audit found nothing to change, and that is the finding.** Longhorn's
crate structure is already the shape it should be. The container was the
anomaly, not the first of a pattern — which is worth knowing before a tag, and
worth not discovering by refactoring something that was right.

## What Not To Do

Do not run a blanket rename or a mechanical crate merge without the compiler
driving it. Card 179 recorded two near-misses from exactly that: a regex meant
to delete two structs took three state primitives with it, and
`PersistedLayoutDocument` was silently renamed because it contains the
substring `LayoutDocument`. Both were caught in under a minute by
`cargo check`, and neither would have been caught by review.
