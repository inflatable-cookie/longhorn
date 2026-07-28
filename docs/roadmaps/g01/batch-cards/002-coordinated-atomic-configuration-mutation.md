# 002 Coordinated Atomic Configuration Mutation

Status: complete  
Owner: Tom  
Completed: 2026-07-28  
Roadmap: g01.002 batch 2  
Governing refs: contracts 001, 004, and 012; research memo 004  
Auto-start next card: no

## Objective

Add capability-confined, patch-based configuration mutation that serializes
all cooperating store instances and processes, preserves the last valid
document on failure, and reports achieved durability without widening the
domain-store boundary.

## Scope

- add a synchronous cross-platform whole-file lock dependency compatible with
  Rust 1.85
- define an injected coordination authority and stable
  `.longhorn/config.lock` location
- layer a process-local mutex over the OS advisory lock
- acquire through non-blocking attempts and a caller-supplied finite deadline
- reread the authoritative domain value after acquisition
- patch, validate, encode, and publish the exact registered envelope
- create a unique same-directory temporary file through the parent capability
- sync the temporary file, rename within the capability, and attempt required
  parent-directory durability
- return typed mutation, coordination, publication, and durability outcomes
- clean up unpublished temporary files
- add same-process, helper-process, timeout, crash, and failure-injection tests

## Public Behavior

- Mutation is available only for missing and current valid file-backed
  outcomes.
- A public mutation accepts a typed closure or patch operation against a fresh
  value. It does not accept unchecked encoded bytes or a blind whole-document
  replacement.
- The store acquires one non-reentrant store-wide guard before its
  authoritative reread and retains it through receipt creation.
- Nested internal work receives the acquired guard. It does not reacquire.
- A finite wait policy is required. Contention returns typed busy or timeout
  state rather than blocking indefinitely.
- Successful serialization uses the existing envelope:

```json
{
  "domain": "example.preferences",
  "schemaVersion": 2,
  "value": {}
}
```

- Failed patch validation or encoding performs no filesystem mutation.
- Failure before replacement leaves the previous valid target intact.
- A successful receipt distinguishes atomic publication from verified durable
  publication.
- In-memory migrated values return `migration-backup-required`.
- Recovery, future-version, and unavailable-authority outcomes remain
  non-writable.
- The persistent lock file is normal infrastructure. Its presence never
  blocks acquisition by itself and it is not deleted as stale.

Internal Rust type and module names may vary. The coordination order,
transaction lifetime, refusal states, envelope, and durability truthfulness
may not.

## Out Of Scope

- debounce queues and shutdown flush
- cross-domain transactions
- pre-migration backup and destructive migration rewrite
- backup archives, retention, restore, and restore receipts
- encrypted backup and secure-store implementation
- project-shared multi-machine mutation or server coordination
- TypeScript bindings, Svelte stores, and Tauri commands
- consumer migration

## Steps

1. Characterize the selected lock dependency under Rust 1.85 and add it to the
   workspace without raising MSRV.
2. Add the injected coordinator, process-local mutex registry, stable lock-file
   open, finite acquisition loop, typed failures, and non-reentrant guard.
3. Extend registered domains with current-value encoding while preserving
   codec ownership and the existing serialized envelope.
4. Add patch mutation over a freshly reread value and refuse every unsafe load
   outcome.
5. Add capability-relative unique temporary creation, file sync, same-parent
   rename, cleanup, permission handling, directory durability, and receipt.
6. Add failpoints and concurrency helpers. Prove preservation, serialization,
   timeout, crash release, confinement, and durability reporting.
7. Run the complete batch validation and record closeout evidence. Compile a
   separate debounce/flush card only after this card closes.

## Acceptance Criteria

- two store instances in one process cannot lose an unrelated field update
- two helper processes serialize mutations through the same stable lock
- a held lock reaches a finite typed timeout
- killing a lock holder permits acquisition without deleting the lock file
- a pre-existing unlocked lock file is harmless
- mutation rereads after lock acquisition and patches that fresh value
- current and missing files publish the exact validated envelope
- migrated, recovery, future, and unavailable outcomes refuse mutation without
  changing source bytes
- each injected failure before rename preserves the previous valid document
- readers observe a complete previous or next document, never a partial one
- unpublished temporary files are cleaned up
- capability confinement still rejects a parent symlink escape
- file permissions are narrow by default
- the receipt states the achieved durability level
- the package graph still contains no Tauri, Svelte, Poodle, Surface, or
  consumer dependency

## Evidence Required

- focused unit tests for coordinator order, non-reentrancy, refusal states,
  encoding, failpoints, cleanup, and receipts
- same-process thread test with unrelated patch updates
- helper-process tests for exclusion, finite timeout, crash release, and
  persistent lock-file behavior
- temporary-root and symlink-escape fixtures
- Rust 1.85 workspace check
- `effigy doctor`
- `effigy test --plan`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `effigy qa`
- batch log with platform durability limits and unresolved-decision notes

## Stop Conditions

- the dependency requires raising Rust 1.85
- stable locking cannot be combined with the existing capability authority
- the implementation must reopen a validated target through an ambient path
- Windows replacement behavior cannot satisfy atomic old-or-new visibility
- directory durability cannot be represented truthfully in the public result
- safe mutation requires changing storage-class authority or the serialized
  envelope
- a migrated value would need destructive rewrite before verified backup exists
- the card expands into debounce, backup, remote authority, or host bindings

## Completion Notes

- added explicit canonicalized coordination authorities and stable
  `.longhorn/config.lock` files
- added one process-local mutex per coordination authority plus `fs4` advisory
  locking
- added finite busy/timeout acquisition and crash-safe handle release
- added domain encoding and fresh-value typed patch mutation
- added capability-relative exclusive temporary creation, file sync, atomic
  rename, cleanup, Unix `0600` mode, and explicit durability receipts
- refused migrated, recovery, future, read-only, unavailable, and
  project-shared authorities without source mutation
- added failpoint, same-process, helper-process, crash, complete-reader,
  permission, and symlink-confinement proofs
- retained Rust 1.85 and a host/UI-free package graph

## Evidence

- 34 passing unit, integration, helper-process, and acceptance tests
- `cargo +1.85.0 check --workspace --all-targets`
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- `effigy doctor`
- `effigy test --plan`
- `effigy qa`

## Next Task

Research and promote the backup/archive decisions in paused card 004.
