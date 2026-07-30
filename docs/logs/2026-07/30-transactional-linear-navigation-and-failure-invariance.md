# Transactional Linear Navigation And Failure Invariance

Date: 2026-07-30
Card: 063
Roadmap: g01.011

## Result

Added immutable revision-bound undo, redo, and entry-id checkout plans to
`longhorn-history`. Execution revalidates the complete plan, invokes one
injected consumer transaction, then commits structural position under the
same exclusive history borrow.

Product apply remains consumer code. Longhorn cannot move history between
apply and commit, and no failed transaction can call the private structural
commit path.

## Protocol

- injected bounded plan id and exact source history revision
- stable entry-id checkout; no renderer index authority
- exact source and target position metadata
- ordered inverse steps toward the retained baseline
- ordered forward steps toward the newest retained state
- configurable bounded navigation depth
- bounded recent-plan ledger for duplicate rejection
- checked source authority, revision, position, direction, target, step ids,
  batch size, and revision successor before product apply
- authoritative receipt with source, committed position, moved ids, and one
  successor revision

An explicit checkout of the current entry is a zero-payload committed
navigation. It advances structural revision and consumes the plan id without
moving entries.

## Apply State Machine

| State | Product model | History |
| --- | --- | --- |
| plan rejected | untouched | untouched |
| apply succeeds | complete target | commits exact target once |
| apply fails, rollback verified | exact source restored | untouched |
| apply fails, rollback fails | terminally uncertain | untouched; no success claimed |
| committed plan repeated | transaction not called | untouched |

Revision exhaustion, missing targets, empty undo/redo, inverse rejection,
oversized checkout, foreign authority, stale source, changed position, and
corrupt private plan shape all reject before product apply.

## Loophole Evidence

The Loophole-shaped fixture retains:

- one-entry undo and redo behavior
- entry-id checkout in both directions
- reverse entry order during multi-entry undo
- forward entry order during redo
- reverse semantic order inside a compound inverse
- current and next undo/redo labels
- exact applied/future movement

Injected failure inside a compound restores the exact model and leaves
history byte-for-byte equal. Injected rollback failure leaves partial product
evidence visible while history remains unchanged. This replaces the donor's
move-before-fallible-apply seam without reducing successful behavior.

The Loophole repository remained read-only. Pulse mutation meaning, runtime
apply, rollback, tempo/cache reconciliation, project persistence, journal
files, recovery, and project versions remain Loophole authority.

## Second Shape

A numeric document fixture proves multi-entry checkout, exact rollback,
rollback failure, stale and duplicate rejection, bounded depth, unknown ids,
and zero-step checkout through the same public API.

## Boundary Audit

- no product model or apply match enters Longhorn
- no index authorizes checkout
- no async runtime, config, bridge, Tauri, Svelte, Poodle, JSON, or donor
  dependency enters the crate
- recent plan ids are transient process state and do not enter structural
  persistence
- formal paged projections remain Card 064 work

## Validation

- `effigy test:history-core`
- 19 history integration fixtures plus one private corrupt-plan fixture
- `cargo +1.85.0 test -p longhorn-core -p longhorn-history`
- `cargo clippy -p longhorn-core -p longhorn-history --all-targets -- -D warnings`
- `cargo doc -p longhorn-history --no-deps`
- `effigy fmt:rust`
- `effigy qa:northstar:g01-history-navigation`
- `effigy qa:northstar:g01-history-cards`
- `effigy qa:docs`
- `effigy qa:northstar`
- `git diff --check`

## Next

Card 064 is ready. Add explicit gesture groups, injected monotonic time, count
and encoded-weight retention, exact pruning evidence, and authoritative
past/current/future pages.
