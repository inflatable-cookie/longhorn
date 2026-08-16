# Consumer-scoped Credential Slots

Date: 2026-08-16
Card: 225
Roadmap: g02.028

## Result

The shared credential store now admits consumer-owned namespace, scope, and
purpose without importing consumer policy into Longhorn. `CredentialSlot` is
an owned validated value with one canonical consumer form:
`consumer:<namespace>:<scope>:<purpose>`.

The built-in constructors retain the exact persisted names
`refresh-token`, `licence-key`, and `backup-identity`. There is no alias,
dual read, or migration path because stored identities did not change.

`CredentialStore` now borrows slots. Memory storage keys by the full slot;
the platform adapter keys by its host-supplied service and the slot's
canonical persisted name. Namespace, scope, purpose, and application-service
differences are all covered as separate identities.

## Validation

- `cargo test -p longhorn-core -p longhorn-credential-keyring -p longhorn-config-age --locked`
- real macOS keychain round trips, locked/unavailable behavior, idempotent
  removal, scoped isolation, and a direct 255-byte account-name probe
- `effigy check:api-reference`, `effigy check:packages`, and
  `effigy docs:rust`
- `POODLE_REPO=<linked Poodle checkout> effigy qa`, using the existing
  linked-Poodle exemption at clean Poodle main
  `70826abf9f59ad2f8d87363338482fe05145652c` while release Card 218 remains
  operator-held
- exact scans found no old enum variants, by-value store signatures, or
  Bovine/Farmyard vocabulary in the shared implementation

## Consumer Handoff

Promote this commit to Bovine Card 128. Bovine owns its namespace, stable
non-secret source discriminator, credential purposes, and any migration from
its product-specific store.
