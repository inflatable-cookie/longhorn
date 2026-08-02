# 115 Soundcheck Storage, Config, And Window Cutover

Status: planned
Owner: Tom
Roadmap: g01.016 batch 2
Governing refs: contracts 003, 004, 009-010, and 012; Cards 113-114
Depends on: Card 114
Auto-start next card: no

## Objective

Move Soundcheck's desktop roots, application settings, and primary-window
placement onto Longhorn while preserving its established product root and
external SQLite authority.

## Repository Scope

- Longhorn: focused adapters, fixtures, evidence, and docs if a generic gap is proven.
- Soundcheck: storage, settings persistence, window host, dependencies, tests, and docs.
- soundcheck-library and Signal: read-only authority checks.

## Scope

- canonical id `com.inflatablecookie.soundcheck`
- `shared-product-root-v1` with stable storage name `Soundcheck`
- fixed locator, legacy discovery, import receipt, and cleanup prohibition
- separate application-settings and window-placement domains
- external SQLite data-class registration without schema ownership transfer
- protected primary-window restore, guarded reveal, event capture, and close flush
- acceptance-environment overrides as explicit injected roots

## Steps

1. Freeze current product root, database, settings, and window receipts.
2. Select the stable-name storage profile through the fixed locator.
3. Register app settings and window placement as separate versioned domains.
4. Import the retained settings/window source with locator-last publication.
5. Bind Soundcheck's primary window to the protected Longhorn host.
6. Preserve minimum size, fitting, hidden startup, and bounded close flush.
7. Keep the SQLite database behind its native external adapter.
8. Prove fresh, established, legacy, interrupted, restart, and rollback starts.

## Acceptance Criteria

- canonical id and `Soundcheck` storage name are distinct stable inputs
- established data is not copied into itself
- settings and window placement have separate structural authorities
- database schema, migration, snapshot, and restore remain soundcheck-library authority
- accepted environment overrides remain explicit and test-confined
- old sources remain until receipt-bound operator cleanup
- one protected primary window reveals only after authoritative restore
- no silent fallback or dual write remains active

## Stop Conditions

- the selected root differs from the recorded stable-name policy
- database placement changes without a native snapshot/restore plan
- a product setting must become a Longhorn type
- window rollback requires the old save worker to remain active

## Next Task

Execute Card 116. Compose Soundcheck's product settings and library recovery
through the shared settings shell.
