# Windowing First-consumer Follow-up

Date: 2026-07-29

## Input

Figmatic's first integration confirmed the windowing boundaries and exposed
repeated adapter work:

- config-domain sink staging and flush
- saved display evidence round trip and restore entry point
- host/scheduler construction cycle
- thread-per-wake scheduling
- repeated timing guesses and empty callbacks
- unclear lifecycle-only single-window path
- silent unit-scale capture corruption on scaled displays

## Outcome

- `SavedWindowPlacement` carries canonical display identity and mapped logical
  evidence.
- `restore_window_placement` reuses canonical saved, intersection, main, and
  deterministic fallback planning and work-area fitting.
- `plan_tauri_window_restore` composes Tauri observation, persistent display
  reconciliation, and pure restore through one adapter entry point.
- `CapturedWindowPlacement::saved` produces the shared record.
- `WindowScaleGeometryMapper` consumes the live Tauri scale per capture.
- `UniformWindowGeometryMapper::identity_scale` names and bounds the 1× case.
- `longhorn-windowing-config::ConfigWindowPlacementSink` supplies coalesced
  staging and coordinated config mutation without adding config to the Tauri
  adapter.
- completed and failed synchronous flush tickets are named constructors.
- `TauriAsyncWindowLifecycleScheduler` runs waits on Tauri's blocking runtime
  pool and binds through `TauriWindowLifecycleHost::shared`.
- `WindowLifecyclePolicy::recommended` is opt-in.
- named no-op close and reporting services make empty policy explicit.
- `assemble_tauri_single_window_lifecycle_host` installs the lifecycle-only
  single-window path and its listener.

## Limits

- A stable `DisplayId` still comes from the display-correlation domain. Raw
  Tauri monitor facts never become canonical identity implicitly.
- `WindowScaleGeometryMapper` cannot prove a coherent mixed-scale global origin
  on every platform. Consumers needing that guarantee inject a platform mapper.
- Figmatic migration remains consumer-owned and follows after this Longhorn
  batch.

## Evidence

- pure saved-display, unplugged-display, deterministic fallback, serde, and
  2× poisoned-size clamp tests
- live-scale 3160×2026 to 1580×1013 mapper regression
- config sink coalescing, target selection, and unrelated-field preservation
- immediate flush-ticket completion tests
- Tauri async scheduler delivery test
- lifecycle-only single-window composition proof
