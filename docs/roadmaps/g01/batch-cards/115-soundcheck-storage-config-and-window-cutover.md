# 115 Soundcheck Storage, Config, And Window Cutover

Status: complete
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

## Result

Soundcheck now selects `shared-product-root-v1` through its canonical locator
and stable `Soundcheck` leaf. Its established `library.db` stays at the product
root through an explicit Data override. Application settings and window
placement are separate Longhorn domains under `config/` and `state/`.

Legacy `settings.json` import verifies a retained backup, splits the two
domains, writes exact digests, refuses target conflicts, and leaves cleanup
unauthorized. SQLite participates through an online backup/restore adapter;
soundcheck-library retains schema and recovery authority.

The predeclared `main` window is now protected `window:primary` state. Restore
starts hidden, fits to current displays with a `320x240` minimum, reveals after
restore plus renderer readiness, captures lifecycle events with 300 ms
debounce, and uses a two-second close bound. The old window save worker is
gone.

Real retained product data also proved a generic gap: same-layout profile
adoption was walking unrelated files. Longhorn now skips that unbounded source
inventory when source and target layout digests match.

## Evidence

- Soundcheck cutover: `c2351a9f7f8de3a5a16ca633f4172ddb10f4665e`
- Longhorn same-layout fix: `ab9cb31a70611a0714b02296016a22f0ae58a615`
- `fixtures/migration/soundcheck-card115/storage-config-window-cutover-v1.json`
- `scripts/verify-soundcheck-card115.ts`
- `effigy qa:northstar:g01-soundcheck-card115`

Isolated proof-root, legacy split, conflict, and locator-last adoption cases
pass. Generic transition recovery covers interruption and rollback. The full
Soundcheck native GUI fresh/restart and rollback pass remains Card 119; no live
user data was used as a test fixture.

## Next Task

Execute Card 116. Compose Soundcheck's product settings and library recovery
through the shared settings shell.
