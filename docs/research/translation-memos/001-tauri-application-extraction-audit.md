# 001 Tauri Application Extraction Audit

Status: complete and promoted  
Owner: Tom  
Updated: 2026-07-27

## Problem

Identify desktop mechanisms worth sharing across current Tauri apps without
turning Longhorn into a Loophole-specific shell or a second Poodle.

## Scope And Method

Scanned `~/Dev/projects` twice:

- tracked-file search for Tauri Cargo and package dependencies
- no-ignore filesystem pass excluding generated/build directories

Five apps were inspected through manifests, Northstar docs, implementation,
tests, and Effigy code graphs:

- `loophole/aura`
- `nucleus/apps/desktop`
- `soundcheck`
- `jetstream` editor
- `acowtancy/bovine-accelerator-desktop`

Loophole, Nucleus, Soundcheck, and Bovine had uncommitted work. This audit is
read-only and describes the observed worktree, not a stable-API endorsement.

## App Findings

| App | Desktop-system evidence | Extraction value |
| --- | --- | --- |
| Loophole Aura | canonical display registry; window fallback and apply loop; hosted Surfaces; eight regions; panel policy; named layouts; cross-window Surface drag; command/keymap/palette | advanced donor; contains the fullest mechanisms but also DAW policy |
| Nucleus Desktop | primary-window restore; project-keyed five-region panel layout; split ratios; panel policy; cross-region drag; native browser panels | proves Surface is optional and exposes duplicated window/layout work |
| Soundcheck | single-window restore in app settings; atomic JSON write; debounced flush; 100+ command calls; progress/listen/cancel flows; isolated plugin inspection | strong persistence, async-operation, and native-integration specimen |
| Jetstream Editor | centralized snapshot bridge; command handler tests; shortcuts; native WGPU view under transparent webview; viewport geometry/input forwarding | strong native-content and bridge specimen; workspace layout is mostly static |
| Bovine Accelerator Desktop | remembered workspace; persisted split/tree/selection state; folder dialog; compact typed invoke facade | greenfield-simple specimen; useful guard against overbuilding |

All five use Svelte 5 and Tauri 2. Four declare Poodle packages; Bovine imports
Poodle source directly. Poodle already owns tabs, dock regions, split views,
drag payload primitives, menus, dialogs, and presentation tokens.

## Direct Duplication

- Loophole `window-drag.ts` and Nucleus `windowChrome.ts` are identical except
  for the error namespace.
- Loophole, Nucleus, and Soundcheck each implement native window capture,
  display fallback/clamping, debounce, and close flush independently.
- Loophole and Nucleus independently implement region/panel persistence,
  allowed-region policy, active tabs, split ratios, and drag affordances.
- Loophole Echo and `nucleus-workspaces` both define display ids, window ids,
  bounds, display fallback, window planning, regions, and panel placement.
- Loophole, Nucleus, Soundcheck, and Bovine each own a local JSON preference
  path with different locking, atomicity, migration, merge, and corruption
  behavior.
- The five renderers contain about 218 direct `invoke` call sites and 17
  `listen` call sites. Most DTOs and lifetimes are handwritten.
- Theme and custom-titlebar bootstrap repeats across the Poodle apps.

## Recommended Shortlist

### Priority 0: extract first

1. **Versioned local state**
   - atomic JSON, migration, locking, partial merge, debounce, flush
   - needed by window and layout packages
   - evidence: four apps
2. **Display and geometry core**
   - canonical records, current availability, correlation seam, clamping,
     logical/physical coordinate types
   - donors: Loophole Echo/Aura, Nucleus, Soundcheck
3. **Window planning and Tauri host**
   - target/fallback resolution, geometry per display, pure apply plan,
     dynamic windows, user/programmatic event attribution
   - donors: Loophole and Nucleus; Soundcheck supplies simple restore behavior
4. **Layout container + region + panel core**
   - opaque container parent, configurable regions, panel policy, ordering,
     active tab, split/collapse persistence
   - donors: Loophole and Nucleus
5. **Svelte/Poodle workspace bindings**
   - stores/actions and Poodle adapters, not new components
   - includes the exact duplicated window-drag helper
6. **Optional hosted Surfaces**
   - Loophole lifecycle, presence, window hosting, ordering, active Surface,
     cross-window drag
   - separate dependency so Nucleus never carries Surface state

### Priority 1: contract after the foundation

7. **Typed Tauri command/event bridge**
   - checked Rust/TypeScript DTOs, listener lifetime, initial snapshot
     handshake, mock handler assembly
8. **Command/action/keymap/palette engine**
   - Loophole has the mature system; Jetstream proves the common shortcut need
   - keep consumer command authority injectable
9. **Application shell bootstrap guidance**
   - custom titlebar, Poodle theme/presentation, visible-on-ready, capabilities,
     command tests, error surfaces
   - likely documentation plus small helpers, not a framework

### Incubate

10. **Long-running operation controller**
    - progress, cancellation, stale request protection, listener cleanup
    - Soundcheck is the main donor; require another consumer
11. **Native content islands**
    - child webviews, isolated plugin windows, or an embedded native renderer
    - common need, incompatible current mechanisms; prototype before API

## Surface Decision

Do not encode one fixed class hierarchy.

Make region/panel layout depend on an opaque layout-container id:

- Nucleus: window id is the container
- Loophole: Surface id is the container; Surface hosting maps it to a window

This retains the full Loophole hierarchy while making Surface a real optional
capability. Cross-window panel movement can target a current container and
region regardless of which composition is active.

## What Stays App-Specific

- panel catalogues and panel bodies
- DAW timeline, mixer, plugin, and transport behavior
- Nucleus project/task/agent resource attachment
- Soundcheck scan/sync/taxonomy and plugin-inspection policy
- Jetstream render loop and viewport semantics
- Bovine content/navigation/editorial models
- product workspace defaults and labels

## Tradeoffs

- More packages and explicit adapters instead of one turnkey shell.
- Some Echo code moves or reshapes rather than being re-exported unchanged.
- Rust authority means renderer-only prototypes need conversion, not copying.
- First extraction is slower because donor characterization and two-consumer
  proof are part of done.

## Validation Needed

- Loophole full-hierarchy fixtures
- Nucleus no-Surface fixtures
- missing-display and corrupt-geometry cases
- concurrent layout/geometry write cases
- cross-language serialization fixtures
- packaged multi-window drag proof

## Promotion

Promoted into:

- `../../architecture/system-architecture.md`
- `../../architecture/system-inventory.md`
- `../../architecture/repo-authority-map.md`
- `../../contracts/002-composable-workspace-hosting.md`
- `../../contracts/003-extraction-and-consumer-migration.md`

## Follow-up

The operator added configuration location, backups, centralized settings,
optional server topology, command palettes, and unified history to the
candidate suite. Their evidence and disposition are preserved in
[`002-shared-desktop-systems-follow-up.md`](002-shared-desktop-systems-follow-up.md).
