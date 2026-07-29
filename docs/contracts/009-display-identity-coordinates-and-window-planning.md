# 009 Display Identity, Coordinates, And Window Planning

Status: active first pass  
Owner: Tom  
Updated: 2026-07-28
Evidence: `../research/translation-memos/003-foundation-boundary-characterization.md`

## Boundary

Longhorn owns machine-local display identity, typed coordinate conversion,
display correlation, window placement, and deterministic planning. Host
adapters own platform observation and native window mutation. Consumers own
window roles, defaults, and adoption policy.

This contract does not require Surfaces.

## Coordinate Spaces

Geometry always names its space:

| Type | Meaning |
| --- | --- |
| `PhysicalPx` | host or native device pixels |
| `ScreenDip` | global, top-left logical desktop coordinates |
| `ClientCssPx` | local webview content coordinates |
| `ScaleFactor` | explicit conversion evidence |

- No public geometry API accepts an untyped tuple or rectangle.
- Durable scale evidence is a positive integer in thousandths; `1000` means
  `1.0`. Zero is invalid.
- Display full bounds and work areas use `ScreenDip`.
- `WindowPlacement` combines an outer origin with an inner content size.
- `LiveWindowMetrics` separately records outer bounds for hit-testing.
- Outer and inner frames are never substituted for each other.
- Physical/logical conversion names its rounding mode and fails typed on
  overflow. No pure API has an ambient rounding default.
- Nearest integer physical-to-DIP-to-physical conversion exposes its
  quantization: error is bounded by `ceil(scale_thousandths / 2000)` physical
  pixels. Identity scale remains exact.
- Host adapters own platform-value conversion and document their selected
  rounding at the boundary.
- A named unit-scale mapper is valid only for an established 1x desktop. Tauri
  capture values are physical; unit scale is not a generic Tauri default.
- Client coordinates reach screen space only through current window metrics.

## Display Identity

- `DisplayId` is an opaque, machine-local Longhorn identity.
- A new known display receives an id once and retains it until explicitly
  forgotten.
- Platform ids, hardware UUIDs, names, position, size, scale, and built-in
  flags are observations and correlation evidence. None is universally
  canonical.
- `DisplayBuiltinStatus` is `unknown`, `built_in`, or `external`. Unknown is
  not serialized or compared as external.
- Canonical ids are not synchronized between machines.
- Known displays may remain absent without losing labels or placement memory.

Correlation runs strongest evidence first:

1. strong platform or hardware key
2. remembered adapter key
3. exact geometry and scale
4. unique weak fingerprint

Every result records its confidence and evidence. Ambiguous weak matches do
not bind automatically or overwrite remembered evidence. A consumer may ask
the user to resolve ambiguity.

New canonical ids come from an injected allocator after correlation. A
platform id, hardware key, fingerprint, or adapter enumeration index never
becomes canonical identity implicitly.

An arrangement signature sorts available canonical ids with full bounds,
work areas, scale, and main-display status. Adapter enumeration order is not
significant.

## Window Planning

Planning is pure and deterministic over known displays, current observations,
saved placement, live windows, and consumer policy.

- A configured display is tried before ordered fallbacks.
- A required primary window falls back through largest useful intersection,
  main display, then the first deterministic available display.
- Required windows are clamped to the target work area and minimum size.
- Temporary fallback does not rewrite the configured home display.
- A settled user move may adopt a new attached display when consumer policy
  permits it.
- Maximized state stores and restores its normal placement separately.
- No available display produces an explicit unavailable plan, not fabricated
  geometry.

The live-versus-desired plan emits explicit operations such as create,
retag, move/resize, maximize, show, focus, and close. Apply generations let
the host suppress feedback from its own mutations.

## Host Boundary

- Tauri monitor and window values are converted at the adapter edge.
- Tauri's baseline monitor API cannot report built-in status. The adapter
  records that fact as unknown unless an injected platform provider supplies
  evidence.
- If the Tauri primary-monitor value cannot identify exactly one available
  observation, main-display attribution fails typed instead of marking an
  arbitrary monitor.
- Tauri physical geometry converts through fixed-point `ScaleFactor` using
  explicit nearest rounding only when one scale defines the complete coordinate
  plane. Invalid, zero, non-finite, or overflowing host values fail typed.
- Mixed-scale global origins require an injected platform coordinate mapper.
  Dividing each monitor origin by its own scale is not a valid generic desktop
  mapping and fails as unavailable.
- A managed-window probe is complete or fails. An unreadable managed window
  cannot disappear from a snapshot and trigger duplicate creation by omission.
- Native apply is ordered and non-transactional. Every attempted operation
  returns a per-operation result. Failure blocks dependent later operations for
  that window, not independent windows.
- A fresh live readback decides convergence. An apply receipt never fabricates
  successful native state.
- Host bookkeeping records the apply generation before issuing a native
  mutation. Event attribution uses that evidence; elapsed time alone is not
  proof of origin.
- Creation delegates consumer-owned URL, title, chrome, minimum-size, and
  capability policy to an injected factory. Longhorn requires only a neutral
  hidden, unmaximized result.
- Retag changes managed host bookkeeping. It does not derive a `WindowId` from
  a Tauri label.
- The host captures user move, resize, and scale changes only after settling.
  Debounce and attribution intervals are explicit policy inputs. Longhorn may
  publish an opt-in recommendation but does not apply it implicitly.
- Capture produces a persistence proposal through an injected sink. The Tauri
  adapter does not depend on configuration storage or mutate product schemas.
- Close and shutdown request bounded explicit flush and return an inspectable
  receipt. Failure remains observable; an event callback never waits
  indefinitely.
- User close is reported to consumer policy. The adapter does not disable,
  delete, or otherwise rewrite desired product state by inference.
- A window remains hidden until placement is applied and the consumer's page
  readiness signal has arrived.
- Screen-point hit-testing uses live outer bounds.
- Window labels are transport identifiers, not domain identity.
- Host failures return typed errors and leave the desired model inspectable.

## Acceptance

- type checks prevent physical, screen-logical, and client-local mixing
- property tests cover round-trip conversion and documented rounding
- Loophole display-correlation fixtures preserve confidence and ambiguity
- Nucleus restore fixtures select saved, intersecting, main, then deterministic
  fallback displays
- window placement tests distinguish outer origin, inner size, and outer bounds
- temporary fallback never silently changes a saved home display
- pure planning imports no Tauri, Svelte, Poodle, or Surface package
- failed or incomplete native observation cannot fabricate an absent window
- apply receipts expose partial failure and verify convergence through readback
- programmatic apply events do not produce durable user-placement proposals
- clean close and shutdown flush; timeout and persistence failure are explicit
- host composition works for one protected window and dynamic multi-window use
  without layout or Surface dependencies
