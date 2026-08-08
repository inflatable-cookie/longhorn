# longhorn-native-content

Pure coordination for native content presented through a desktop window but
not owned by renderer DOM.

## Boundary

The crate owns:

- bounded island and kind identity
- nonzero attach generations and independent desired/observed revisions
- one generation-bound logical `WindowId`
- mechanism capabilities, desired state, and fresh observations
- client-viewport to physical-geometry conversion
- immutable ordered plans and exact partial receipts
- content-size proposals and explicit host-destruction invalidation

It does not own Tauri, native handles, browser policy, plugin hosts, GPU
renderers, product input, Svelte, Poodle, or outer-window placement.

## Coordination Flow

1. Construct validated `MechanismCapabilities`, `DesiredUpdate`, and
   `DesiredState`.
2. Create one `NativeContentCoordinator` per island.
3. Request an immutable `ApplyPlan`.
4. Execute operations in dependency order in a selected adapter.
5. Submit sparse `StepExecution` evidence through `receipt` while the complete
   plan cursor is current.
6. Admit a complete fresh `ObservationUpdate`. Observation, not call success,
   decides convergence.

Content-size requests use `ContentSizeProposal`; their receipts authorize a
later desired update and never mutate desired geometry directly.

Call `host_destroyed` before admitting later native events from a destroyed
host. The call is idempotent for the current generation. Reattachment requires
the exact next generation.

## Mechanism Geometry

| Mechanism | Viewport operation | Input route |
| --- | --- | --- |
| `ChildView` | complete child bounds | usually `NativeDirect` |
| `IsolatedWindow` | content size only | usually `NativeDirect` |
| `BackingSurface` | presentation and interaction clip | usually `RendererForwarded` |

The isolated-window path never owns outer placement. Backing storage bounds
remain distinct from the viewport clip.

## Renderer Protocol

Enable `bindings` only for checked Rust-to-TypeScript generation. The protocol
uses exact version 1 and typed request, result, snapshot, event, proposal,
decision, observation, and receipt shapes. `NativeContentProtocolHost` is one
pure island-scoped authority seam for later adapters; it performs no native
operation and emits no framework event itself.

Every renderer connection receives a new `NativeContentClientEpoch`. This is
not `AttachGeneration`: renderer remounts advance the former, while host
replacement advances the latter. Mutations carry request identity, client
epoch, island identity, and expected authority cursor. Stale sessions and
revisions fail without mutation.

The generated client lives in `@inflatable-cookie/longhorn-native-content`. Its optional Tauri
assembly maps the same checked shapes over narrow injected commands and one
changed event.

## Prototype Differences

The production package derives from the Card 082 evidence without preserving
its API as authority:

- domain ids and `NativeContentRevision` use `longhorn-core`
- attach generations and plan steps reject zero, including during decode
- a host change requires a new generation
- mechanism descriptors declare their active input route; unsupported desired
  or observed routes fail before mutation
- detach is not reissued while an observed detach is already in flight
- host destruction explicitly invalidates the current generation
- apply receipts are admitted only while island, desired revision, observed
  revision, and generation all match
- invalid desired-state decoding re-runs capability validation

The retained prototypes remain evidence, not compatibility targets.
