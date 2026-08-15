# 224 Credential-store Conditional Write

Status: complete — the primitive was investigated and refused, with evidence
Completed: 2026-08-15
Owner: Tom
Roadmap: g02.023 batch 3 (follow-through on Card 210)
Governing refs: contract 004 (§ noninteractive identity authority); Card 210
Depends on: Card 210 (which built the mechanism this hardens)
Auto-start next card: no

## Objective

Close the first-run race in `StoreBackupEncryption`, or establish that it
cannot be closed and narrow it as far as the available primitives allow.

## Why this exists

Card 210's `identity()` is read-then-generate-then-store, which is not atomic
across processes. Two instances launched together, on a machine where the slot
is still empty, both read `None`, both generate, and both store. Last write
wins. The loser kept the identity *it* made, so every archive it encrypted
before exiting was unreadable by the identity that survived — silent backup
loss, discovered only at restore.

The obvious fix is a compare-and-swap on `CredentialStore`: store-if-absent,
returning what was already there. This card was opened to add it.

## The finding: there is no conditional write to call

`CredentialStore` is a Longhorn trait, so adding a method is free. Honouring
it is not — and the shipping backend cannot:

- `longhorn-credential-keyring` composes `keyring` 4.1.6, whose surface is
  `keyring-core` 1.0's `CredentialApi`. That trait offers `set_password` /
  `set_secret` / `get_secret` / `delete_credential` and nothing else. There is
  no create-only, no conditional update, no compare-and-swap. `set_secret`'s
  own documentation specifies unconditional replacement.
- Bypassing the crate does not rescue it. macOS could use `SecItemAdd` and its
  `errSecDuplicateItem`, but Windows' `CredWrite` has no create-only flag at
  all — so a compare-and-swap does not exist as a *cross-platform* primitive,
  which is the only kind this trait can declare.

A method on `CredentialStore` with a default implemented as retrieve-then-
store would be behaviourally identical to the code it replaced, while reading
like a guarantee. That is the advisory-guarantee shape memo 023 criticised
elsewhere in the tree (client-side event filtering, the fetch size limit), and
it would be worse here because the backend Longhorn itself ships would be the
one violating it.

**Decision: no conditional write on `CredentialStore`.** Recorded on the trait
so it is not re-litigated.

## What landed instead

Generation now reads the slot back and adopts whatever the store names,
instead of keeping what it made. The losing process encrypts to the winner's
identity, so the two converge rather than one writing doomed archives. A store
that reports a successful write and then reads back empty is refused as
`Unavailable` — it is not a store, and encrypting to a secret that went
nowhere is the failure this card exists to prevent.

The remaining window is the read-back itself: two rival writes landing between
one process's write and its read-back still leave that process superseded.
That is strictly narrower than the original — which was open for the whole
process lifetime — and it is as far as unconditional writes reach.

## Scope

- `crates/longhorn-config-age/src/store.rs` — the read-back and its tests
- `crates/longhorn-core/src/credential.rs` — the refusal, recorded on the trait
- contract 004 — state the convergence property and its limit

## Do Not

- Add `store_if_absent` with a racy default. The point of the investigation is
  that the default *is* the current behaviour wearing a guarantee's name.
- Solve it with a lockfile or a single-instance assumption. Longhorn does not
  own the consumer's process model, and contract 004's backup path must work
  in whatever process the consumer runs it in.

## Acceptance Criteria

- [x] the losing process adopts the surviving identity rather than its own
- [x] a store that keeps nothing is refused, not encrypted to
- [x] both properties fail their tests when the read-back is removed
- [x] the absence of a conditional write is recorded with its evidence, on the
  trait and on this card
- [x] contract 004 states the property and its remaining limit

## Evidence Required

- `keyring-core` 1.0 `CredentialApi` having no conditional write
- the adoption test failing without the read-back
- `effigy qa` green

## Stop Conditions

Reopen if a future `keyring-core` gains a conditional write, or if a consumer
needs a credential slot whose correctness depends on true mutual exclusion —
that would be a contract-scope question, not a provider detail.
