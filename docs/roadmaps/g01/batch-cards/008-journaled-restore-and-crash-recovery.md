# 008 Journaled Restore And Crash Recovery

Status: planned after card 007
Owner: Tom
Roadmap: g01.002 batch 3
Governing refs: contracts 001, 004, and 012; research memo 006
Auto-start next card: no

## Objective

Publish a staged multi-domain restore with exact rollback payloads, durable
operation journal, verified terminal state, crash recovery, and truthful
cross-file visibility semantics.

## Scope

- private exact current-state rollback set
- verified policy-compliant pre-restore safety backup
- durable phased restore journal and recovery marker
- existing single-file atomic publisher for each selected domain
- complete post-publication verification
- rollback after every injected ordinary failure
- retained rollback material and write refusal after unverified rollback
- startup/operation recovery that rolls back rather than resumes
- active-restore and recovery-required load states
- coordinated multi-domain load-set
- pre-migration backup integration and destructive migration rewrite
- terminal restore, rollback, recovery, and migration receipts

## Public Behavior

No multi-file atomic visibility claim is allowed.

Under the store coordinator, execution captures old bytes and absence, verifies
a retained safety backup, publishes the journal, replaces domains one by one,
and verifies the full target. Failure restores and verifies the full old set
before returning.

A crash or failed rollback blocks normal mutation. Recovery uses private
rollback material and defaults to restoring the old set. It does not complete
the originally requested destructive restore after confirmation context was
lost.

Single-domain files remain atomically old or new. Loads detecting the journal
return unavailable. A load already in flight may complete against either
complete file. Cross-domain readers requiring one generation use the new
coordinated load-set.

Older normal configuration may be destructively rewritten only through the
same safety-backup and journal boundary.

## Out Of Scope

- age encryption
- external/custom restore adapters
- Tauri lifecycle and settings UI
- consumer migration

## Acceptance Criteria

- success verifies every selected target
- every injected publication or verification failure restores and verifies
  every selected source
- failed rollback leaves journal and payloads and blocks later writes
- crash at every journal phase recovers the exact old set before mutation
- recovery is idempotent
- safety backup is verified and pinned until terminal cleanup
- rollback does not depend on an optional encryption key
- active restore and recovery-required loads are typed
- coordinated load-set cannot observe a mixed generation
- ordinary single-domain readers never observe partial bytes
- pre-migration rewrite cannot occur without verified old-source backup
- receipts distinguish success, verified rollback, and recovery required

## Stop Conditions

- rollback depends only on a user export or encrypted archive
- journal publication is not durable before the first live replacement
- startup can ignore an unresolved journal
- ordinary failure can return before rollback verification
- implementation claims a portable multi-file atomic rename
- the card expands into encryption or consumer adapters

## Next Task

Run after card 007 closes. Then activate the age encrypted archive adapter.
