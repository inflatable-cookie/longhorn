# Retained Notification Ledger And Operation Observation

Date: 2026-07-31
Card: 079
Status: complete

## Changed

- Added the pure finite `longhorn-notifications` authority.
- Added bounded notification, source, replacement, producer-token, cause, and
  semantic-action identity plus ledger revision and insertion sequence.
- Added explicit add, replace, seen, dismiss, clear, and retention transitions.
- Added count and canonical encoded-weight retention with exact removal
  receipts, protected-record policy, and newest-first bounded pages.
- Added feature-gated terminal-operation observation and idempotent publication.
- Added Loophole render and unrelated reliability fixtures over one ledger.

## Decisions

- Retained read/removal truth is independent from toast lifetime.
- Add always names a fresh record. Title is presentation only. Replacement uses
  a unique source/key pair; publication replay uses a producer token.
- Severity does not silently select retention. Consumers opt into protected
  treatment, and unsatisfiable limits reject atomically.
- Operation policy receives immutable committed evidence and returns zero or
  one bounded draft. It cannot mutate operation authority.

## Evidence

- Exact stale-state, duplicate, already-seen, clear-target, retention, and
  numeric-overflow rejection leaves the ledger unchanged.
- Seen, dismiss, clear, and prune receipts remain distinguishable.
- Count and weight pressure prune oldest standard records and report all
  removals; protected data and the newly admitted record are never hidden.
- A zero-capacity ledger forces publication failure after a Loophole-shaped
  operation is terminal; the operation catalogue remains byte-for-byte equal.
- The default crate graph contains only `longhorn-core`. The optional feature
  adds `longhorn-operation`; neither graph contains bridge, Tauri, async,
  Svelte, or Poodle dependencies.

## Next

Execute Card 080. Generate checked notification clients and add isolated
Svelte/Poodle projections plus fresh semantic-action admission.
