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
| Tauri adapter crates | 11, totalling 13,433 lines |
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

## Finding 2 — Eleven Tauri Crates Could Be One

`longhorn-tauri-bridge`, `-command`, `-config`, `-history`, `-history-tree`,
`-native-content-child-view`, `-notifications`, `-operation`, `-settings`,
`-transfer`, `-windowing`.

They are independent: exactly one depends on another. Most expose ten to
fourteen public items over two to six hundred lines. `-windowing` is the
outlier at 134 items and 6,077 lines.

Consumers already take subsets — 1, 4, 6 and 8 of them across the four
applications — which is the argument for keeping them apart. Cargo features
serve that need with one dependency line instead of eight, and a consumer that
adds a domain edits a feature list rather than hunting the right crate name.

**What this does not buy:** the CI and release gates run `--workspace` and name
no crate individually, so consolidation saves nothing there. The gain is
consumer ergonomics and eleven fewer manifests, not gate maintenance. Worth
being accurate about, because "fewer crates" is easy to justify with reasons
that turn out to be false.

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
2. Collapse the Tauri crates behind features. Largest reduction, mechanical,
   and the consumers are all in this portfolio.
3. Test the `-config` counter-argument, then fold if it does not hold.
4. Leave `longhorn-config` alone unless something needs `backup` on its own.

## What Not To Do

Do not run a blanket rename or a mechanical crate merge without the compiler
driving it. Card 179 recorded two near-misses from exactly that: a regex meant
to delete two structs took three state primitives with it, and
`PersistedLayoutDocument` was silently renamed because it contains the
substring `LayoutDocument`. Both were caught in under a minute by
`cargo check`, and neither would have been caught by review.
