# g01.015 Loophole Full-hosting Migration

Status: blocked on `g01.014` and optional Surface packages  
Owner: Tom  
Updated: 2026-07-27  
Governing refs: contracts 002 and 003

## Outcome

Migrate Loophole to Longhorn's full display, window, Surface, region, panel,
settings, and command composition without moving DAW authority.

## Batches

### 1. Seam freeze

- map Echo/Aura crates and renderer modules to Longhorn packages
- capture display correlation, window apply, Surface lifecycle, panel, drag,
  settings, command, and history fixtures
- choose ownership for generic Echo crates

### 2. Foundation cutover

- configuration/backup and machine state
- display/window planner and Tauri host
- layout core and client bindings

### 3. Optional systems

- Surface hosting and cross-window movement
- settings and command/input projections
- history linear-kernel adoption only if g01.011 promotes it

### 4. Cleanup and proof

- remove active donor duplicates
- packaged multi-display/multi-window proof
- record product adapters left in Loophole

## Acceptance

- full hierarchy remains `display -> window -> Surface -> region -> panel`
- Pulse, Signal, Aura workflows and panel catalogue remain Loophole-owned
- no dual active implementation remains after each cutover
- behavior deltas are deliberate and documented

