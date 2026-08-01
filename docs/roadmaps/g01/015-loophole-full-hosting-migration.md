# g01.015 Loophole Full-hosting Migration

Status: ready for bounded compilation; no donor writes admitted
Owner: Tom  
Updated: 2026-08-01
Governing refs: contracts 002, 003, and 008

## Outcome

Migrate Loophole to Longhorn's full display, window, Surface, region, panel,
settings, and command composition without moving DAW authority.

g01.014 and the optional Surface packages are complete. A fresh read-only
Loophole audit must now freeze current authority and compile bounded cards
before this roadmap can become active.

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
- history linear-kernel adoption after Cards 062-067 and g01.011 closeout
- explicit import of existing Pulse history; retain journal recovery and
  cross-session undo
- branch mode remains disabled during this migration; Card 069 promotes the
  semantics, while g01.017 must implement and prove the optional tree layer
  before a separate Loophole adoption decision

### 4. Cleanup and proof

- remove active donor duplicates
- packaged multi-display/multi-window proof
- record product adapters left in Loophole

## Acceptance

- full hierarchy remains `display -> window -> Surface -> region -> panel`
- Pulse, Signal, Aura workflows and panel catalogue remain Loophole-owned
- no dual active implementation remains after each cutover
- behavior deltas are deliberate and documented
- all 83 Pulse mutation variants remain Loophole-owned and usable through the
  shared policy/apply seam
- project versions remain separate from undo branches
