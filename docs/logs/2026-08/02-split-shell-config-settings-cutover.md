# Split-shell Config And Settings Cutover

Date: 2026-08-02
Roadmap: g01.016
Card: 120
State: complete; Card 121 ready

## Result

Split-shell now uses Longhorn's smallest useful desktop composition. Canonical id
`com.example.split-shell` is also the storage leaf under
`platform-native-v1`; no stable-name override exists. One versioned UserConfig
domain owns the selected workspace and three presentation fields.

The former whole-file writer is gone. Selection validates before mutation;
presentation and settings changes patch only their admitted fields. Unknown
preference keys survive. Durable store-wide coordination replaces direct
publication.

Legacy `app_config_dir/workspace.json` import keeps exact source bytes, a
backup, and a receipt. Startup re-verifies source, backup, target, paths, and
digests. Interrupted import resumes from backup. Conflicts and drift fail
closed. Cleanup and dual-write rollback remain forbidden.

The modal settings registry contains only Workspace and Storage. Workspace
root is read-only, navigation ratio is staged, and storage diagnostics is the
only configuration capability. The existing Poodle SplitView remains
product-owned presentation, not a Longhorn layout document.

## Evidence

- prior Split-shell: `b713644e08be384d20879b0ac31f7899589c8a9b`
- Split-shell cutover: `6afc8da9f4ccc2989541312560eaeb4a154057d2`
- Longhorn selected source: `b25a46f6ab5d06fa1cc06867f2cae61712b7c162`
- config/settings fixture:
  `fixtures/migration/split-shell-card120/config-settings-cutover-v1.json`
- private artifact fixture:
  `fixtures/migration/split-shell-card120/private-artifact-admission-v1.json`
- verifier: `effigy proof:split-shell-card120`

The Split-shell commit changes exactly 19 runtime, manifest, lock, and renderer
paths. The pre-existing 19 dirty docs/CHANGELOG paths remain uncommitted and
untouched by the migration.

## Validation

Split-shell `effigy qa` passes: 60 Rust tests and zero Svelte errors or warnings.
The renderer production build and macOS debug app bundle pass. Tests cover
invalid selection, publication failure invariance, fresh defaults, restart,
exact legacy backup, unknown-field retention, interrupted resume, and receipt
drift.

Produced-artifact proof passes for the exact four TypeScript and six Rust
Longhorn selections plus five Poodle packages. Isolated installs resolve one
Svelte 5.56.8 runtime. Layout, Surfaces, transfer, bridge, commands, history,
operation, notifications, and native content are absent. No live operator
storage or package registry was touched.

## Next

Execute Card 121. Exercise settings lifetime and isolated storage paths, prove
previous-build readback and rollback, audit duplicates and authorities, then
close Split-shell.
