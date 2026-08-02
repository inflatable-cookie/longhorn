# Soundcheck Settings, Backup, And Recovery Cutover

Date: 2026-08-02
Roadmap: g01.016
Card: 116
State: complete; Card 117 ready

## Result

Soundcheck now composes one sealed Longhorn settings registry and one modal
Poodle shell. Agent Review, Custom Tags, Vendors, Composer, and Keepsake stay
Soundcheck renderers. Storage diagnostics, automatic-backup inventory, and
restore inspection use the shared config pages. Empty transition, create,
export, retention, ordinary-restore, and recovery capabilities are not
admitted.

Agent Review now publishes model and reasoning effort as one checked immediate
apply unit over `soundcheck.app-settings`. Soundcheck still owns the schema,
defaults, normalization, model discovery, and wording. The renderer save loop
and direct mutation command were removed. Longhorn's shell now omits staged
Apply/Cancel actions from immediate and read-only pages.

Automatic SQLite backups remain soundcheck-library authority. The host lists
its native backup ids newest first, runs SQLite `quick_check`, hashes exact
archive bytes, and permits inventory selection only. Inspection binds backup
id, archive digest, and a fresh online snapshot digest of the current database
into one confirmation. Execution rechecks all three before calling the native
restore path.

The adapter preserves native safety backup, validation, migration, and
rollback. Fresh online snapshot evidence distinguishes verified success,
verified rollback, and recovery-required terminals. Only verified success
emits `soundcheck://library-restored`; the existing app invalidation refreshes
library state. No renderer filesystem command remains.

## Exact Evidence

- prior Longhorn: `ea566ea4d7c9727641401048e91f55d545f0168c`
- Longhorn shell support: `31022048064ac0b49345b7df6a19f6653e37a70d`
- prior Soundcheck: `c2351a9f7f8de3a5a16ca633f4172ddb10f4665e`
- Soundcheck cutover: `af715e2fafca8c14d91bd352740f671ff41f24e9`
- retained soundcheck-library: `40418635674d598f7e311bf311c3165300edd35b`
- Soundcheck Cargo lock SHA-256:
  `4183ea91ae56e88788a10fdf265f4a83df04643795d82815ab948d4345898fd2`
- Soundcheck Bun lock SHA-256:
  `2460e9db82353ff302bae5bf7b9572c3a283b9e26bae836ba832cc7ebd78c13f`
- fixture:
  `fixtures/migration/soundcheck-card116/settings-backup-recovery-cutover-v1.json`
- verifier: `effigy proof:soundcheck-card116`

## Validation

Soundcheck passes its production renderer build, 28 renderer tests, and 105
app-library tests with two local-Codex tests ignored. Longhorn passes all 20
settings-shell tests and Svelte checking. The exact verifier checks commits,
both Soundcheck locks, page/capability admission, one-domain apply authority,
native SQLite evidence, removed direct commands, and retained sibling
authority.

Full native fresh/restart, successful restore, rollback, and recovery-required
GUI proof remains Card 119. This batch did not mutate live Soundcheck data.

## Next

Execute Card 117. Move plugin-scan structural lifecycle onto Longhorn operation
authority while keeping scan execution, reports, retry meaning, and recovery
in soundcheck-library.
