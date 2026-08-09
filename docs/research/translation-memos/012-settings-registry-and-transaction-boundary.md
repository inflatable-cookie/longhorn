# 012 Settings Registry And Transaction Boundary

Status: complete and promoted
Owner: Tom
Updated: 2026-07-29
Extends: `002-shared-desktop-systems-follow-up.md`

## Prompt

Revalidate the first-pass settings contract against the implemented
configuration, client, Svelte, Poodle, and shell foundations before compiling
g01.008.

## Evidence

### Longhorn

- `longhorn-config` provides registered typed domains, fresh coordinated patch
  mutation, bounded debounce, explicit flush, backup, restore, storage-layout
  profiles, and profile transition.
- Configuration values are consumer types. A shared settings system cannot
  turn product schemas into Longhorn wire authority.
- Existing checked clients and Svelte state already define listener-first
  connection, explicit status, request reconciliation, and exact teardown.
- Card 041 proves that app shells can share lifecycle and Poodle integration
  without sharing one application frame.

### Loophole

- The current modal has App, Appearance, Hardware, Keybindings, and Workspace
  pages over public Poodle dialog and navigation primitives.
- App settings use host snapshots plus set/reset commands. Hardware and plugin
  isolation use specialist host commands. The page body is not reducible to
  one generic form schema.
- The Echo settings protocol already separates registry metadata, configured
  and effective values, set/reset commands, apply metadata, and changed events.
- The donor does not prove a reusable cross-domain atomic transaction.

### Soundcheck

- One dialog mixes immediate preferences, model discovery, product taxonomy,
  optional integrations, and backup/restore workflows.
- Its latest-wins save loop is useful renderer evidence but not a durable
  transaction contract.
- Backup and restore require richer confirmation and receipt state than an
  ordinary preference field.

### Split-shell

- The app has one small preference domain and no settings shell.
- It proves that registry and shell packages must not require layout, Surface,
  server, command, or history systems.

## Boundary Decisions

- Settings is an optional system, not part of the app-shell root.
- A pure Rust registry owns stable ids, composition metadata, authority
  snapshots, commands, and receipts. It owns no product schema or renderer.
- A config adapter binds one apply unit to one registered configuration domain.
  It uses fresh coordinated mutation and host-issued authority tokens.
- One-domain apply is failure-atomic. A multi-domain page uses separate apply
  units or an explicit consumer transaction authority. The shell never labels
  sequential writes atomic.
- Immediate and staged are mutation timing modes. Restart-required is an
  activation result and may accompany either mode.
- Managed policy is host authority. Projections distinguish configured and
  effective values, provenance, constraints, and editability.
- Page registration is sealed per host generation. Missing optional modules
  create no navigation entries. Runtime outages remain visible page states.
- Custom product pages register renderer keys and standard session controllers.
  Longhorn does not infer UI from arbitrary product JSON.
- Search and deep links resolve stable page ids. Capabilities control
  composition and availability, not Tauri security.
- The shell owns navigation, session switching, dirty guards, apply/cancel
  coordination, and errors. Poodle owns visual primitives.
- Storage profile, backup, restore, and diagnostics are shared modules over the
  exact g01.002 plans and receipts. They do not weaken confirmation or recovery
  rules for UI convenience.

## Package Consequences

- `longhorn-settings`: pure ids, registry, authority protocol, snapshots,
  commands, and receipts
- `longhorn-settings-config`: registered config-domain apply units and shared
  storage/backup/restore modules
- `longhorn-tauri-settings`: narrow command/event host assembly
- `@inflatable-cookie/longhorn-settings`: checked protocol, client, registry projection, session
  controller, and optional Svelte/Poodle subpaths
- `@inflatable-cookie/longhorn-config`: generated storage, backup, restore, and recovery client
  used only by composed shared pages

The root settings protocol does not depend on layout, Surfaces, commands,
history, or backend topology.

## Deferred

- command-aware keybinding semantics remain g01.010
- backend connection pages remain g01.009
- server-synchronized settings and remote conflicts remain g01.009
- schema-generated product forms need separate evidence
- live module installation and registry mutation are not v1 behavior

## Promotion

Promoted into:

- `../../architecture/system-architecture.md`
- `../../architecture/system-inventory.md`
- `../../architecture/package-topology.md`
- `../../contracts/005-settings-and-system-registration.md`
- `../../roadmaps/g01/008-settings-registry-and-shell.md`
- Cards 042-048
