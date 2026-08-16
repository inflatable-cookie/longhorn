# 225 Consumer-scoped Credential Slot Value

Status: ready
Owner: Longhorn maintainers
Roadmap: g02.028
Governing refs: contract 021; contracts 001, 003, 004, and 012
Depends on: Card 224 complete
Auto-start next card: no

## Objective

Replace the closed `CredentialSlot` enum with the validated slot identity in
contract 021, retaining exact built-in keyring names while admitting isolated
consumer namespace/scope/purpose slots.

## Scope

1. Implement the owned validated slot and typed validation error in
   `longhorn-core`.
2. Preserve named built-in constructors and their three exact persisted names.
3. Change `CredentialStore` and every implementation/caller to borrow a slot.
   Do not leave enum aliases or dual APIs.
4. Make memory storage key by the complete slot value and keyring storage use
   only the canonical persisted name.
5. Add boundary, collision, service-isolation, replacement, removal, and
   locked/unavailable tests for built-in and consumer slots.
6. Update API/reference, architecture, guides, and release-facing inventories
   that still describe the store as licence-only or the slot vocabulary as
   closed.

## Acceptance

- [ ] Built-in persisted names are byte-identical before and after the change.
- [ ] Namespace, scope, purpose, and service differences produce distinct
      entries.
- [ ] Every malformed or overlong segment and total name fails before backend
      access.
- [ ] Memory and mock/real-platform keyring contract suites pass.
- [ ] `rg` finds no old enum variant call sites or product vocabulary in the
      shared implementation.
- [ ] `effigy qa` passes.

## Out Of Scope

Bovine settings, Farmyard credentials, secret provisioning, key rotation,
enumeration, conditional write, and external consumer-repo mutation.

## Stop Conditions

Stop if preserving existing keyring names requires a dual read, if a supported
platform rejects the frozen canonical name, or if source isolation needs
secret-derived identity or entry enumeration.

## Next Task

Return the promoted Longhorn commit to Bovine Card 128 adoption.

