# 002 Shared Desktop Systems Follow-up

Status: complete and promoted  
Owner: Tom  
Updated: 2026-07-27  
Extends: `001-tauri-application-extraction-audit.md`

## Prompt

Preserve the systems named after the initial audit and determine whether
configuration, settings, backend topology, commands, and history belong in
Longhorn's runway.

## Added System Candidates

| System | Cross-app case | Current evidence | Disposition |
| --- | --- | --- | --- |
| configuration storage | every desktop app needs stable local roots and writes | four independent JSON implementations | foundation |
| backup and recovery | settings, layouts, and machine state can be lost or corrupted | Loophole recovery; atomic writes in Loophole and Soundcheck | foundation capability |
| settings shell | multiple apps need one discoverable place for registered settings | Loophole modal and keybinding tab; scattered controls elsewhere | shared shell plus app pages |
| optional server topology | Nucleus already separates desktop and server concerns; other apps may follow | local Tauri hosts, Nucleus server crates, Loophole process boundaries | contract before transport |
| command/action registry | palette, menus, shortcuts, and automation need one catalogue | mature Loophole action/input stack; Jetstream shortcuts | priority after bridge |
| unified history | undo, history panels, recovery, and branching may recur | mature Loophole linear stack; branch research | research and prototype |

## Configuration Evidence

- Loophole `echo-configuration::ConfigStore` has an injected filesystem
  adapter and atomic JSON writes. Machine/windowing state is versioned.
- Soundcheck stores product settings and window geometry together beside its
  database. It serializes access, debounces geometry, and replaces through a
  temporary file.
- Bovine uses Tauri `app_config_dir`, but writes a single unversioned
  `workspace.json` directly.
- Nucleus has several domain stores plus desktop-local UI state. Its server
  topology proves that not all durable state shares one authority.
- No inspected app provides a complete cross-domain backup inventory,
  checksummed restore flow, secret exclusion policy, or future-schema safety.

The reusable unit is not merely `read_json` and `write_json`. It needs a
storage taxonomy, injected roots, domain registration, schema migration,
atomic transactions, backup policy, and recovery reporting.

## History Evidence

Loophole's live `pulse-history` crate currently provides:

- serializable typed mutations with inverse and no-op logic
- compound mutations
- automatic coalescing
- 750 ms gesture grouping
- bounded undo and redo stacks
- persisted stack snapshots
- jump-to-position and lightweight UI snapshots
- a separate runtime apply layer for Pulse-specific mutations

The mutation enum and apply implementation are DAW-specific. The stack
mechanics are reusable in shape, but are coupled to that enum today.

Forkable or branching history is not live donor behavior. Loophole research
recommends branching as opt-in and explores event sourcing, checkpoints, and
recovery, but the current stack clears redo on a new mutation. Longhorn must
not advertise a history tree by extracting the linear stack unchanged.

## Promotion

Promoted into:

- `../../specs/001-shared-desktop-system-suite.md`
- `../../architecture/system-architecture.md`
- `../../architecture/system-inventory.md`
- `../../contracts/004-configuration-storage-backup-and-recovery.md`
- `../../contracts/005-settings-and-system-registration.md`
- `../../contracts/006-command-action-and-input.md`
- `../../contracts/007-optional-backend-topology.md`
- `../../contracts/008-history-kernel-boundary.md`

## Residual Research

- platform-specific backup and restore semantics
- multi-process locking and remote configuration authority
- settings transaction behavior across independent domains
- history payload strategy: inverse operations, patches, snapshots, or events
- branch-tree retention, checkpointing, migration, and recovery performance
- whether async operations and notifications share a generic lifecycle

