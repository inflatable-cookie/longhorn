# 002 Composable Workspace Hosting

Status: active first pass  
Owner: Tom  
Updated: 2026-07-27  
Evidence: `../research/translation-memos/001-tauri-application-extraction-audit.md`

## Problem

Loophole needs hosted Surfaces. Nucleus found that the same layer duplicated
its project and panel navigation. A shared system must support both without
parallel implementations.

## Contract

### Hosting composition

- display inventory and window planning are independent of layout topology
- regions belong to an opaque layout-container id
- panels belong to regions
- a consumer may bind a window directly as the layout container
- the optional Surface module may bind Surfaces as layout containers and host
  them in windows
- core region/panel APIs do not require Surface types

### Surface module

When enabled, it owns:

- stable Surface identity
- ordered window hosting preferences
- active Surface per window
- Surface lifecycle and presence policy
- Surface-to-window resolution

It does not own panel bodies or consumer workflow state.

### Region and panel policy

- region ids and families are consumer-configurable
- panel definitions carry allowed-region and instance policy
- rejected placement never mutates durable state
- active panel, order, size, and collapse are deterministic
- missing policy does not become unrestricted placement

### Authority

- Rust is authoritative for durable resolution and mutation
- renderer projections do not invent fallback or active-selection truth
- Svelte adapters bind state to Poodle public APIs
- Poodle remains authoritative for component interaction and visuals

### Drag

- same-webview movement may use Poodle HTML5 drag payloads
- cross-webview/window payloads carry ids and coordinates only
- the host re-resolves against current state before mutation
- a no-Surface consumer targets window container + region directly
- a Surface consumer targets Surface container + region, then resolves hosting

## Acceptance

- one fixture resolves `window -> region -> panel`
- one fixture resolves `window -> surface -> region -> panel`
- both fixtures share the same region/panel resolver
- neither fixture imports the other's optional topology types
- Loophole and Nucleus can map their current ids without product-domain
  dependencies in Longhorn

## Open Contract Gaps

- exact cross-window target selection
- persistence envelope
- logical/physical coordinate boundary
- Rust/TypeScript serialization authority
