# Authoritative Surface Lifecycle And Persistence

Date: 2026-07-29
State: complete implementation batch

## Outcome

- completed Card 029
- added expected-revision create, duplicate, rename, activate, reorder, move,
  and close commands
- validated current state before cloning a private candidate
- committed exactly one checked revision or returned the exact unchanged
  document
- required caller-supplied fresh Surface and layout-container ids
- added explicit read-only layout-container inventory evidence
- kept `longhorn-surfaces` independent of `longhorn-layout`
- copied generic Surface metadata and hosting policy without layout or product
  contents
- made declared target host promotion and insertion explicit
- preserved target active state when its member remained present
- selected source and close fallback deterministically
- made empty-window allow/reject policy consumer-owned
- returned layout-container cleanup intent without executing cleanup
- added the `longhorn-surfaces-config` crate
- added exact registered domains, strict raw shape, migrations, backup policy,
  recovery, and coordinated complete-document publication
- proved Surface, layout, and window persistence remain separate
- made Card 030 ready

## Lifecycle Contract

Every request carries a `SurfaceRequestId`, expected `SurfaceRevision`, and one
strict command. Admission order is:

1. validate current Surface state
2. compare the expected revision
3. check revision advance
4. apply to a private candidate
5. normalize and revalidate
6. publish one complete successor

Rejection returns its stable code, diagnostic, current revision, and the exact
source document. No replay cache or silent repair is part of this card.

Create and duplicate accept an external `LayoutContainerInventory`. The
inventory is evidence only. Surface mutation can confirm existence and
unbound status but cannot inspect, clone, create, or delete layout contents.
The persistence adapter derives the inventory from the supplied
`LayoutDocument` before entering Surface publication.

## Active Transition Table

| Command state | Committed active state |
| --- | --- |
| reorder; active remains a member | unchanged |
| move; target active remains a member | unchanged |
| move; moved Surface was source active | member at former source index, else previous final |
| move; no remaining primary source member and empty allowed | none |
| move; no remaining primary source member and empty rejected | reject unchanged |
| close; closed Surface was active | member now at former index, else previous final |
| close; active was another remaining member | unchanged |
| close; no remaining member and empty allowed | none |
| close; no remaining member and empty rejected | reject unchanged |

Move promotes an already-declared candidate window to first host preference.
Source fallback considers remaining first-preference members. Close removes
all candidate memberships and considers all remaining declared members.

## Persistence Contract

`RegisteredSurfaceDomain` accepts:

- exact `DomainDescriptor`
- validated normalized default
- explicit `SurfaceLimits`
- consumer migration hook
- include or reasoned-exclude backup policy

The current raw value contains one complete Surface document and denies
unknown fields. Older shapes load only through ordered migration. Future,
corrupt, or incompatible bytes remain available in typed recovery evidence.
Structural mutation reloads fresh state under the configuration coordinator,
rechecks the expected revision, and immediately publishes one complete
document.

## Evidence

The Surface contract suite covers:

- all seven lifecycle commands
- fresh Surface and container identity checks
- unknown and already-bound container rejection
- duplicate host, target, insertion, and reorder failures
- stale, overflow, invalid-current, and invalid-candidate invariance
- one-revision success
- generic-only duplicate metadata
- active transition rules
- allow/reject empty-window policy
- cleanup intent without layout deletion

The configuration suite covers:

- missing default and current publication
- corrupt, future, and incompatible recovery
- explicit old-shape migration and missing migration
- same-revision concurrent writers admitting exactly one
- backup inclusion and reasoned exclusion
- distinct Surface, layout, and window files and values

## Package Report

| Package | Ordinary dependencies |
| --- | --- |
| `longhorn-surfaces` | `longhorn-core`, serde |
| `longhorn-surfaces-config` | config, core, layout, surfaces, serde, JSON |

The pure package imports no layout body, config, filesystem, Tauri,
TypeScript, Svelte, Poodle, or product authority.

## Validation

- 17 focused Surface contract tests pass
- 6 focused Surface configuration tests pass
- scoped warnings-denied Clippy passes
- no new god-file warning
- Rust 1.85 workspace all-target check passes
- full Effigy QA passes
- `git diff --check` passes

## Remaining Limits

- no native window composition
- no durable adoption of temporary fallback hosts
- no cross-domain atomic cleanup
- no generated TypeScript or renderer adapter
- no transfer session, lease, or drag behavior
- no consumer migration

These remain assigned to Cards 030-035 and later adoption roadmaps.

## Posture

`strict-ready`

## Next

Start Card 030: compose resolved Surfaces through the existing runtime-generic
window host and prove full and no-Surface dependency shapes.
