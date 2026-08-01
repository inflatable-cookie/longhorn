# Private Native-content Coordination Prototype

Date: 2026-07-31
Card: 082
Roadmap: g01.013

## Result

Implemented a standalone, non-publishable Rust prototype for native-content
coordination. One desired/observed vocabulary now represents child views,
isolated native windows, and backing surfaces without merging their host
implementations.

The prototype owns bounded island and kind identity, revision and attach
generation, host-window binding, typed viewport and scale, explicit presence,
visibility, focus intent, input routing, lifecycle planning, operation
receipts, and content-size proposals. Successful plans never fabricate native
observation.

## Three-shape Proof

- child-view plans map the semantic viewport to physical child bounds
- isolated-window plans map it to content size and leave outer placement to
  windowing
- backing-surface plans keep storage bounds distinct from render and input
  clipping
- consumer visibility reasons remain explicit across every mechanism
- focus requests remain distinct from observed native focus

Nucleus-, Soundcheck-, and Jetstream-shaped traces pass through the same pure
API. No donor payload, arbitrary JSON value, or raw native handle enters the
model.

## Failure And Freshness

Every renderer measurement, adapter observation, and lifecycle update names
an attach generation. Stale or future generations reject before mutation.
Desired-state replacement checks its expected revision. Generation advances
are exact and cannot replace a live attachment.

Plans are dependency ordered. Receipts distinguish applied, failed,
dependency-skipped, and not-attempted work. Unknown visibility and focus stay
unknown; a successful apply still requires fresh native observation.

## Boundary Audit

The prototype is a nested workspace outside Longhorn's root workspace. Its
normal dependency graph contains only `longhorn-core` and `serde`. It imports
no Tauri, browser, plugin, GPU, Svelte, or Poodle code and creates no public
package or compatibility promise.

The three mechanisms remain independently packaged work. Child construction
and security, native helper/process ownership, and GPU rendering are not part
of this prototype.

## Validation

- 21 contract tests pass across three traces, generations, geometry,
  receipts, proposals, and boundary audits
- strict clippy and formatting checks pass
- 1x and 2x checked conversions, rounding, zero viewport, and overflow pass
- stale revision/generation failures preserve exact state
- the dependency and forbidden-symbol audits pass
- `effigy qa` passes the complete repository and artifact-proof suite

## Next Task

Execute Card 083. Apply the frozen child-view trace through an isolated,
private packaged Tauri prototype without importing isolated-window or
backing-surface mechanisms.
