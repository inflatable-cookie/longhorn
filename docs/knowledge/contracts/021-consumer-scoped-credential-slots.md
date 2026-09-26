# 021 Consumer-scoped Credential Slots

Status: active compiled boundary
Owner: Longhorn maintainers
Created: 2026-08-16
Depends on: contracts 001, 003, 004, and 012
Affects: `longhorn-core`, `longhorn-credential-keyring`, credential consumers

## Problem

`CredentialStore` is shared host plumbing, but `CredentialSlot` is a closed
enum containing only Longhorn's licence and backup names. A consumer needing
two instances of the same secret purpose, such as one credential set per
registered publishing source, cannot use the shared store without either
colliding entries or adding product vocabulary to Longhorn.

## Contract

- `CredentialSlot` becomes an owned, validated value rather than a closed
  product enum.
- Longhorn keeps named constructors for its built-in slots. Their persisted
  names remain exactly `refresh-token`, `licence-key`, and `backup-identity`.
- A consumer-scoped slot is constructed from `namespace`, `scope`, and
  `purpose` segments. Longhorn validates and joins them as
  `consumer:<namespace>:<scope>:<purpose>`.
- Each segment is lowercase ASCII, begins and ends with an alphanumeric
  character, may contain internal hyphens, and is 1 through 64 bytes. The
  complete persisted name is at most 255 bytes. Empty, uppercase, whitespace,
  control, separator, traversal-shaped, or overlong input is rejected before
  a backend call.
- Namespace, scope, and purpose meaning remains consumer-owned. Longhorn does
  not know Bovine, Farmyard, a source UUID, a tenant, an account, or a product
  credential kind.
- Consumers use a stable non-secret scope discriminator. A secret, raw token,
  filesystem path, display label, mutable URL, or user-entered prose must
  never become slot identity.
- `CredentialStore` accepts the validated slot by reference. Memory and
  platform backends use only its canonical persisted name.
- Platform service identity remains host-supplied. Service plus canonical slot
  name is the complete keyring identity; slots cannot cross application
  services.
- `retrieve` keeps the existing truth boundary: absent is `Ok(None)` and
  locked, denied, corrupt, or unreachable is `Unavailable`.
- Store, replace, retrieve, and remove never serialize or log secret values.
  Slot validation failures may expose the rejected segment category, never a
  secret-bearing input value.
- No wildcard, prefix scan, list-all, bulk remove, fallback alias, legacy dual
  read, or implicit slot migration is admitted.

## Compatibility And Migration

This is a deliberate pre-1.0 source break. Existing built-in call sites move
to the named constructors and borrowed slot arguments in one change. Persisted
keyring entry names do not change, so existing licence and backup credentials
remain readable without copying or re-storing secrets.

Consumer slots are additive storage identities. A consumer owns any migration
from a previous product-specific store; Longhorn does not probe foreign names.

## Required Evidence

- validation fixtures for every segment and total bound
- exact built-in persisted-name fixtures
- distinct namespace, scope, purpose, and application-service fixtures
- memory-store and keyring mapping conformance over consumer-scoped slots
- locked-is-not-absent and idempotent-remove regression coverage
- repository-wide migration from enum variants with no compatibility alias
- forbidden product vocabulary scan across Longhorn source

## Stop Conditions

Stop if a consumer needs secret-derived slot identity, enumeration, cross-app
sharing, conditional write, or platform-specific naming behavior. Each changes
the shared security boundary and needs a separate contract.

