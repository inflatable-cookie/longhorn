# 120 Bovine Config And Settings Cutover

Status: complete
Owner: Tom
Roadmap: g01.016 batch 3
Governing refs: contracts 003-005, 010, 012-013; Cards 113-114 and 119
Depends on: Card 119
Auto-start next card: yes

## Objective

Move Bovine's single workspace preference document onto the smallest useful
Longhorn config/settings composition without adding layout, Surface, service,
command, history, or native-content systems.

## Repository Scope

- Longhorn: focused minimal-composition fixtures, evidence, and docs.
- Bovine: only config, settings, dependency, test, and migration files named by
  the card after unrelated-work clearance.
- Poodle: read-only exact artifact use.

## Scope

- canonical id `com.acowtancy.bovine-accelerator`
- `platform-native-v1` with no stable-name override
- versioned workspace-preference domain and safe partial mutation
- import of existing `app_config_dir/workspace.json`
- selected workspace root plus presentation fields without content mutation
- minimal modal settings registry and shared storage diagnostics
- exact private Poodle artifacts instead of unpacked source aliases

## Steps

1. Recheck Bovine status and exclude every unrelated dirty docs path.
2. Freeze the exact `workspace.json` schema and current error behavior.
3. Register one versioned domain under canonical native-platform roots.
4. Import the retained unversioned file with an exact receipt.
5. Replace whole-file writes with coordinated safe mutation and explicit flush.
6. Compose one minimal settings shell with only admitted pages.
7. Replace private Poodle source aliases with the admitted private artifact graph.
8. Prove invalid selection, save failure, restart, and rollback behavior.

## Acceptance Criteria

- unrelated Bovine work is byte-for-byte preserved
- no stable storage name overrides the canonical id
- selected repository content remains external and read-only
- Rust retains repository, hierarchy, search, editorial, validation, and Git authority
- unknown content shapes remain visible
- the minimal settings shell has no empty optional pages
- no layout, Surface, bridge, operation, notification, command, history, or native-content edge resolves
- legacy preference cleanup requires an exact receipt and operator authority

## Stop Conditions

- migration-owned files overlap unrelated dirty work
- Longhorn would interpret content, navigation, editorial, or Git semantics
- the selected content root would move into managed app storage
- a Poodle source alias is required for the clean artifact graph

## Result

Bovine now resolves `platform-native-v1` from canonical id
`com.acowtancy.bovine-accelerator` with no stable storage leaf. One versioned
UserConfig domain owns workspace root and presentation preferences through
coordinated durable mutation. Unknown preference fields survive round trips.

Legacy `app_config_dir/workspace.json` import publishes and re-verifies exact
backup, source, target, and receipt digests. Interrupted imports resume from
the verified backup. Conflicts fail closed. The source remains in place and
cleanup is not authorized.

The modal registry contains only Bovine Workspace and shared Storage pages.
Workspace root is read-only; navigation ratio is the sole staged setting.
Only storage diagnostics are admitted. The renderer uses four Longhorn and
five public Poodle packages. Six Longhorn Rust crates resolve. Layout,
Surfaces, transfer, bridge, commands, history, operation, notifications, and
native content remain absent.

## Evidence

- Bovine cutover: `6afc8da9f4ccc2989541312560eaeb4a154057d2`
- fixture: `fixtures/migration/bovine-card120/config-settings-cutover-v1.json`
- refreshed artifacts:
  `fixtures/migration/bovine-card120/private-artifact-admission-v1.json`
- verifier: `effigy proof:bovine-card120`

All 19 pre-existing dirty paths remain confined to `docs/` and
`CHANGELOG.md`. The cutover commit changed exactly 19 runtime, manifest, lock,
and renderer paths. It changed no authored content or unrelated docs.

## Validation

Bovine `effigy qa` passes with 60 Rust tests and zero Svelte diagnostics. The
renderer production build and macOS debug app bundle pass. Focused cases cover
invalid selection, publication failure, restart, interrupted import, exact
unknown-field retention, and completed-receipt drift. The isolated artifact
proof reproduces all selected Longhorn and Poodle sets with one Svelte runtime
and no package-manager publication. Live operator storage was not opened.

## Next Task

Execute Card 121. Close Bovine as the minimal real consumer and prove the
smaller optional graph.
