# Lossless History Donor Fixtures And Typed Foundation

Date: 2026-07-30
Card: 062
Roadmap: g01.011

## Result

Added `longhorn-history`, a pure generic Rust history foundation. It owns
structural linear state and successful record transitions. Consumers retain
payload meaning, product apply, rollback, persistence codecs, storage,
journals, checkpoints, and recovery.

## Foundation

- bounded history, entry, kind, group, and plan ids
- distinct checked history revision and insertion sequence
- hard-bounded labels plus configured count and label limits
- typed entry payload with no JSON or consumer enum
- consumer inverse, no-op, and coalescing policy
- validated oldest-to-current applied state and farthest-to-next future state
- strict imported-state identity, order, revision, count, and metadata checks
- record-after-success with expected history revision
- explicit added, replaced, removed, and ignored-no-op outcomes
- exact future-id evidence for divergent linear records
- checked overflow; no ambient clock, randomness, or saturating counter

## Loophole Evidence

The fixture retains successful typed inverse, automatic adjacent coalescing,
stable coalesced entry identity, coalesced removal, full applied/future shape,
next undo/redo labels, divergent redo clearing, default limit 100, and
structural state round-trip.

The crate README freezes the parity and correction table. Deliberate
corrections are injected ids, checked counters, explicit standalone no-op
handling, and visible future compatibility work. Donor pruning waits for Card
064. Persistence encoding and transition journals wait for Card 065.
Navigation waits for Card 063 so history never moves before fallible product
apply.

The Loophole repository was read-only. Its 83-variant mutation enum, runtime
apply, tempo/cache reconciliation, project snapshots and versions, autosave,
journal file policy, and recovery remain product authority.

## Second Shape

A typed document fixture uses set-title, insert, and remove payloads through
the same public API. It proves inverse, coalescing, separate insertion, policy
rejection, and exact failure invariance without editor, Tauri, Svelte, Poodle,
config, bridge, async runtime, project file, or journal dependencies.

## Boundary Audit

`longhorn-history` has one normal dependency: `longhorn-core`. It imports no
Serde, JSON, config, bridge, Tauri, async runtime, Svelte, Poodle, or donor
package. `LinearHistoryState<P>` is structural state only; Card 062 defines no
wire format or codec.

## Validation

- `effigy test:history-core`
- 11 history foundation tests plus core tests
- `cargo +1.85.0 test -p longhorn-core -p longhorn-history`
- `cargo clippy -p longhorn-core -p longhorn-history --all-targets -- -D warnings`
- `cargo doc -p longhorn-history --no-deps`
- `effigy fmt:rust`
- `effigy qa:northstar:g01-history-foundation`
- `effigy qa:northstar:g01-history-cards`
- `effigy qa:docs`
- `git diff --check`

## Next

Card 063 is ready. Add revision-bound undo, redo, and entry-id checkout
planning plus checked commit after atomic consumer apply.
