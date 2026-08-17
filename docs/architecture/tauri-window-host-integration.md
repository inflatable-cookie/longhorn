# Tauri Window Host Integration

`longhorn-tauri-windowing` composes checked observation, desired/live apply,
event attribution, settled capture, guarded reveal, bounded flush, and
teardown. The same `assemble_tauri_window_host` function runs with Tauri's
native and mock runtimes.

It does not own product window builders, layout, Surfaces, configuration
schemas, renderer state, or Poodle components.

## Assembly

Build predeclared windows in the application, assign stable `WindowId` values,
then assemble once during Tauri setup:

```rust
let host = assemble_tauri_window_host(
    app.handle(),
    lifecycle_policy,
    lifecycle_services,
    [PredeclaredTauriWindow::new(main_id, main_window)],
    Some(HostWindowHandle::new("main")?),
)?
.into_parts()
.0;
```

`PredeclaredTauriWindow` binds caller-owned stable identity to a native window.
The Tauri label remains an opaque transport handle. Use
`with_initial_normal` when the boot window may start maximized and the
consumer has retained normal placement.

Assembly validates all stable ids, labels, and the protected handle before
listener registration. The app-managed initialization slot serializes setup.
A repeated call returns the existing host with `Reused`; it does not register
another listener or replace policy and services.

## Injected Services

`TauriWindowLifecycleServices` requires explicit:

- monotonic clock
- wake scheduler
- geometry mapper
- complete live capture backend
- placement sink with bounded flush acknowledgement
- user-close callback
- lifecycle reporter
- native reveal backend

These are host mechanics. The placement sink adapts captured proposals into
the consumer's durable schema and write authority.

Use `TauriAsyncWindowLifecycleScheduler` with
`TauriWindowLifecycleHost::shared` or either assembly helper. Shared
construction binds a weak wake target after the host exists; consumers do not
need an `OnceLock` cycle or thread-per-wake policy. Use
`NoopWindowUserCloseHandler` and `NoopWindowLifecycleReporter` when those
policies are deliberately empty.

`longhorn-tauri-windowing` stays independent of configuration.
`longhorn-windowing-config::ConfigWindowPlacementSink` is the optional bridge
for a consumer-owned `ConfigDomain`. It coalesces captures per window and
publishes requested targets through one coordinated mutation. The consumer
supplies only the projection into its domain value.

## Geometry Scale

Tauri capture reads physical positions and sizes. Unit scale is not a Tauri
default. `UniformWindowGeometryMapper::identity_scale` is valid only for an
established 1× physical/logical desktop. Using it on a 2× display persists
double-sized logical geometry.

`WindowScaleGeometryMapper` uses the live Tauri window or monitor scale for
each conversion. It keeps sizes correct when a window crosses differently
scaled displays. Its global origin is valid only where the platform's physical
origin divided by current scale forms one coherent logical desktop. Mixed-scale
global origins otherwise require an injected platform mapper.

`LogicalLayoutMapper` is that mapper for macOS and Linux. It exists because a
laptop plus an external monitor is an ordinary arrangement that
`UniformScaleMapper` refuses outright, which stopped a consumer completing
hidden-window restore before startup could read saved state.

It converts every display and window through its own scale. That works on
macOS and Linux for one reason: both lay the desktop out in logical units and
report physical facts as those units times each object's scale, so dividing
returns the original layout exactly.

Windows does not work that way, and is the case the general prohibition exists
for. Its virtual desktop is a real physical-pixel plane read from the OS. Put a
3840x2160 display at 200% beside a 1920x1080 display at 100% and the second
monitor's physical origin is x=3840; divided by its own scale that stays 3840,
where a coherent logical layout wants 1920 — a phantom gap between monitors
that touch. Windows needs a mapper that reads its own layout, and until one
exists a mixed-scale Windows desktop keeps the typed refusal.

Mappers name the hosts they are valid for rather than selecting a strategy at
runtime. Longhorn states host differences instead of erasing them.

Restore through `SavedWindowPlacement` and `restore_window_placement`.
The record carries optional canonical `DisplayId` plus mapped display evidence.
Resolution tries the saved display, useful intersection, main display, then
canonical deterministic fallback, and fits size into the selected work area.
`plan_tauri_window_restore` is the complete adapter entry point: it observes
current displays, reconciles the persistent `KnownDisplayRegistry`, returns the
updated reconciliation, and runs pure restore planning.

## Simple Host

A Nucleus-shaped host supplies one protected predeclared window and
`NoWindowFactory`. The host computes capabilities without `Create`.

When an app needs only settled capture, persistence, and shutdown flush, use
`assemble_tauri_single_window_lifecycle_host`. It installs the native event
listener. Do not forward the same events through a second
`window.on_window_event` callback.

Use `assemble_tauri_window_host` when the app also needs desired/live diff
apply, guarded reveal, protected-primary policy, or dynamic creation.

Restore flow:

1. Observe managed windows.
2. Build `WindowDiffInput` with desired and live evidence.
3. Call `for_hidden_restore`.
4. Call `host.apply` with `NoWindowFactory`, a native mutation backend, and
   fresh readback.
5. Signal renderer protocol readiness with `host.mark_page_ready`.

Reveal requires both fresh converged hidden-placement readback and page
readiness. Either signal may arrive first.

## Dynamic Host

A Loophole-shaped host uses the same assembly for its protected main window
and supplies a `TauriWindowFactory` on apply. The factory owns URL, title,
chrome, minimum size, product metadata, and label choice. It must return a
hidden, unmaximized neutral slot.

After successful creation, the composed registry installs lifecycle handling
before later move, resize, show, or focus operations in that plan. Successful
create receipts end with `RegistryInsert` then
`InstallLifecycleListener`. Registration failure removes the slot from managed
state and reports stable id, created handle, and exact failed call.

Keep dynamic labels within a declared family such as `workspace-*`. Stable
`WindowId` values remain independent of that label convention.

## Capabilities

Rust-hosted create, move, resize, show, focus, close, observation, and
persistence calls do not require matching renderer permissions. Add renderer
permissions only for operations invoked from webview code.

Copy the narrow example matching the host:

- [protected main](../../crates/longhorn-tauri-windowing/examples/capabilities/protected-main.json)
- [protected main and dynamic workspaces](../../crates/longhorn-tauri-windowing/examples/capabilities/protected-main-and-workspaces.json)

Both examples grant only custom-titlebar drag. The dynamic example names
`main` and `workspace-*` explicitly. Add unrelated application permissions in
separate consumer capability policy; do not turn these examples into a broad
default.

`tauri_host_capabilities(false)` excludes native creation.
`tauri_host_capabilities(true)` adds it. Host apply replaces caller capability
claims with this executable set.

## Failure And Receipt Rules

- initialization failures identify registry validation or listener
  registration; listener failures retain stable id and native handle
- planning and generation failures return `TauriWindowHostError::Apply`
- each attempted operation retains generation, stable id, optional native
  handle, completed calls, and exact failed call
- probe/readback failure is stored in `ApplyReadback::Failed`
- event and capture failures remain lifecycle receipts or typed lifecycle
  errors
- sink refusal, sink failure, timeout, and disconnect remain
  `WindowFlushOutcome` values
- reveal failure stays beside the complete apply receipt

Apply temporarily removes the registry from shared state. Injected factory,
native mutation, readback, reveal, capture, user-close, and sink calls do not
run while the registry mutex is held.

## Shutdown And Teardown

Call `host.teardown` during application shutdown. The first call:

1. rejects concurrent apply
2. captures current managed windows where required
3. sends one sorted aggregate bounded flush
4. deactivates all installed listener targets
5. returns `TornDown` with shutdown evidence

Later calls return `AlreadyTornDown` without another flush.

Tauri's window-event registration API does not return an unlisten handle.
Teardown therefore deactivates callbacks and clears their managed targets.
Callbacks retaining a weak host reference become no-ops after teardown or
host drop.

## Mock Proof

The package integration suite covers:

- Nucleus: protected single-window hidden restore, capture, reveal, shutdown
  flush, repeated initialization, and idempotent teardown
- Loophole: protected main plus dynamic workspace creation and listener
  registration
- Soundcheck: minimal predeclared host and two-second close flush
- initialization, planning, factory, native apply, probe/readback, unknown
  event, sink request, sink completion, timeout, disconnect, capture, and
  reveal failure surfaces
- Tauri capability parsing and exact permission contents

Packaged native runtime behavior remains Card 022 evidence.
