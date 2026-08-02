# Loophole Surface Lifecycle And Hosting Cutover

Date: 2026-08-02
Roadmap: g01.015
Card: 108
State: complete

## Changed

- Added the `loophole.regional-surfaces` MachineState domain beside the
  independent window-placement and regional-layout domains.
- Imported enabled regional Surfaces with deterministic external layout
  bindings and complete owner-first fallback-window preferences.
- Kept focused-panel Surfaces as explicit Loophole attachments. They have no
  regional layout container, so registering them would invent a false binding.
- Routed create, duplicate, close, activate, reorder, move, and project restore
  through checked Longhorn Surface validation and registered publication.
- Used the checked Longhorn mutation engine for representable create, close,
  activate, and whole-Surface move transitions. Multi-change restore and named
  layout transactions publish one normalized complete replacement.
- Coordinated product workspace persistence, regional layout publication, and
  regional Surface publication with exact rollback on failure.
- Preserved focused-panel promotion/return, fallback placement, last-window
  policy, and native fullscreen as Loophole behavior.
- Added migration backup and target-integrity receipt files. Interrupted receipt
  publication can recover from retained source evidence.
- Added failed-window-provision cleanup and propagated project-restore failure
  instead of silently continuing with divergent topology.

## Authority Boundary

Longhorn now owns generic regional Surface topology, lifecycle validation,
registered persistence, host preferences, and checked whole-Surface movement.
Loophole retains presence, focused eligibility and attachment, panel fallback,
last-window close, fullscreen, and project meaning. Echo remains only in the
compatibility snapshot vocabulary and current display/window resolver adapter;
it no longer mutates Surface lifecycle or project restore state.

Renderer listener ordering, transfer-session leases, drop-zone composition,
and Poodle drag orchestration remain Card 109 work.

## Evidence

- donor fixture:
  `fixtures/surfaces/loophole-registered-authority-v1.json`
- focused attachment exclusion test
- checked whole-Surface move inference and registered revision test
- exact topology-persistence failure rollback test across product, layout, and
  Surface documents
- project restore, window provision, focused promotion/return, and transient
  fullscreen coverage in Aura's shell suite

## Validation

- Aura native library: 196 passed
- Aura Rust formatting and diff checks: passed
- Loophole `effigy validate`: passed; Svelte 0 errors and 0 warnings
- Longhorn donor fixture and Surface binding checks: passed
- Northstar spine, index, roadmap, heading, and Loophole path checks: passed

## Next

Execute Card 109. Replace renderer lifetime and drag orchestration with checked
Longhorn clients, listener-first transfer sessions, and public Poodle seams.
