# g02.042 Exclusive Admission Lease

Owner: Tom
Created: 2026-09-22
Governing refs: contract 018 (`Exclusive Admission Lease`)
Depends on: g02.041
UI classification: none

## Outcome

`UpdateGate` acquires a host-supplied exclusive admission lease and holds it
from authorization through `apply`, so no new conflicting work starts during the
replacement. Quiescence stays the precondition; the lease is the barrier.

## Context

Contract 018 was amended 2026-09-22. `UpdateGate::authorize` currently returns
`Approved | Deferred` from a point-in-time quiescence receipt; nothing holds an
admission barrier through the replacement. Desktop contract 038 requires an
exclusive mutation admission barrier through replacement. This lane follows
`g02.041`, whose staged protocol defines the interval the lease spans.

## Ready-State Rubric

- [x] The operator approved the contract 018 amendment (2026-09-22).
- [x] The consumer requirement is settled (Desktop contract 038).
- [x] The lease shape is confirmed: a Longhorn host-supplied admission-authority
      trait, not an external host-only seam.
- [x] UI classification is none.

## Work

1. A host-supplied admission-authority trait: the host grants an exclusive
   lease when nothing conflicting is in flight, and the lease blocks new
   conflicting work for its lifetime.
2. `UpdateGate` / `InstallAuthorization`: acquire the lease and hold it from
   authorization through `apply`. A lease that cannot be acquired is a deferral
   carrying its reason, never a failure and never a silent bypass.
3. The Tauri seam and generated projections expose the held-lease/deferral
   state.
4. Tests: a held lease refuses new work through `apply`; an unacquirable lease
   defers with its reason; the lease releases only after `apply` returns.

## Acceptance and review oracle

| Invariant | Required proof |
| --- | --- |
| Lease held through apply | new conflicting work is refused for the whole critical section |
| No gate bypass | an unacquirable lease defers; nothing installs |
| Release after apply | the barrier lifts once `apply` returns |
| Host-supplied | the admission authority is injected; Longhorn learns no application operation |

Validation uses the focused `longhorn-update` selectors plus `check:bindings`,
then `effigy qa`. An independent exact-head review must inspect the lease's
lifetime, not prose.

## Stop conditions

Stop if holding the lease requires the gate to know an application's
operations, or if any design would let an update install with an unacquired
lease. Escalate to Chatterbox.

## Evidence

On completion, record: the trait, the gate change, the seam binding, and the
test results.
