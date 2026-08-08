# Settings Composition

Status: canonical for g01.008
Updated: 2026-07-29
Governing contracts: 004, 005, 010, 012, and 013

## Boundary

Longhorn owns settings identity, sealed composition, checked mutation,
configuration operations, session state, and thin Poodle integration. The
consumer owns product schemas, renderer selection, specialist commands,
application framing, and any transaction spanning more than one apply unit or
configuration domain.

The shell is optional infrastructure. It is not an application framework.

## Bootstrap

1. Register stable modules, sections, pages, renderers, scopes, apply units,
   and capabilities.
2. Seal the registry before opening a settings host.
3. Create one `SettingsSession` per mounted host.
4. Load the sealed registry and authoritative scope snapshot.
5. Resolve page renderers in the consumer.
6. Reveal the modal, window, or panel only after the session is ready.
7. Stop the session on unmount so every listener is released.

Registration order does not define UI order. Explicit order and stable IDs do.
Missing optional modules contribute no pages, permissions, or dead navigation.

## Composition Shapes

| Shape | Host | Product composition | Longhorn packages |
| --- | --- | --- | --- |
| Bovine | modal | one staged Preferences page | core, settings |
| Soundcheck | window | Audio plus shared Storage, Backups, Restore & Recovery | core, settings, config |
| Loophole | panel | immediate Application; staged Appearance; consumer Hardware and Keybindings | core, settings |
| Nucleus | window | one General page; no Surface or backend pages | core, settings |

These shapes share the registry/session/shell contract. They do not share an
application frame, product renderer, or optional-system dependency.

## Registration And Rendering

A page declaration names its renderer but does not import it into the Rust
authority. The consumer maps renderer IDs to Svelte content. That keeps
hardware probes, keybinding editors, audio models, and other product behavior
in the app that owns it.

Shared configuration pages live in `@inflatable-cookie/longhorn-config/poodle`. Register only
the modules the app exposes:

- Storage projects the active profile, canonical identity, resolved paths, and
  transition diagnostics.
- Backups lists and creates explicit backup publications.
- Restore & Recovery inspects first, collects per-domain choices, plans the
  exact operation, asks for confirmation, executes, and shows the terminal
  receipt.

The Poodle shell supplies visual structure and interaction primitives through
public Poodle APIs. Longhorn does not fork Poodle components.

## Mutation Rules

Immediate and staged describe interaction timing. They do not describe
activation.

- immediate sends one checked mutation when the user changes the value
- staged keeps edits local until Apply
- activation requirements come from the authoritative receipt
- reset is a separate checked command
- managed or unsupported values are projected as non-writable
- stale authority returns a conflict and refreshed snapshot
- invalid intent, policy veto, and recovery-required publish nothing

One apply unit may publish atomically through its configured authority.
Multiple dirty units receive separate receipts unless the consumer supplies an
explicit broader transaction authority. The shell must not label separate
receipts as one atomic save.

## Configuration Operation Rules

Inspection and planning do not publish. Backup creation, storage transition,
restore execution, and recovery are explicit mutations.

Restore terminals stay distinct:

- `succeeded`: the planned publication completed
- `rolledBack`: publication failed and the prior state was restored
- `recoveryRequired`: rollback could not restore ordinary operation

Recovery-required state blocks ordinary settings mutation. Recovery uses its
own command and receipt. UI copy must preserve those distinctions.

## Capability Posture

Grant only commands used by the registered composition. Every settings host
needs settings read, settings mutation when it has writable units, and event
listen/unlisten. Soundcheck-style recovery composition adds config read plus
the exact storage, backup, and restore mutation grants. Specialist Loophole
commands remain consumer permissions.

Do not grant Surface, backend, command-palette, layout, or configuration
permissions merely because another settings composition uses them.

## Artifact Boundary

Consumer proof installs packed TypeScript archives and one exact Poodle
artifact set into clean roots. It rejects workspace links, sibling source
aliases, duplicate Svelte runtimes, unexpected Longhorn packages, and
capability drift.

The private Rust proof inventories each crate with `cargo package --list`,
archives that inventory, unpacks it into a clean temporary workspace, and
builds a consumer offline. This proves source completeness and dependency
direction before release. Cargo registry-normalized `.crate` installation
remains a release-lane concern because the private interdependent crates do
not yet exist in a registry.

## Consumer Migration

1. Inventory existing settings pages, persistence owners, confirmation flows,
   and app-specific commands.
2. Assign stable IDs without changing product schema ownership.
3. Bind each writable page to the narrowest existing configuration domain and
   apply unit.
4. Move session and shell mechanics first. Keep product renderers in place.
5. Add shared Storage, Backups, or Restore modules only when the app exposes
   those operations.
6. Run old and new authority reads against the same fixtures before cutover.
7. Migrate persisted data through the storage-profile transition machinery;
   never silently adopt a new path.
8. Remove the old dialog only after checked mutation, failure, recovery,
   teardown, and capability evidence passes for that consumer.

Donor implementations are evidence. A migration must not copy donor-specific
authority into Longhorn.

## Proof

Run:

```sh
effigy proof:settings-composition
```

The proof sources live in
`examples/settings-composition-proof/`. The closeout evidence is recorded in
`docs/logs/2026-07/29-settings-composition-proof-and-closeout.md`.
