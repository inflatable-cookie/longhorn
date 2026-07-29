# Surface And Window Host Composition Conformance

Date: 2026-07-29
State: complete implementation batch

## Outcome

- completed Card 030 and the Surface foundation checkpoint
- added `longhorn-surface-windowing` as an optional pure composition crate
- projected resolved participating Surface hosts and existing placement
  outcomes to plain `DesiredWindow` inputs
- retained complete placement evidence without placing geometry in Surface
  state
- ignored direct non-Surface window outcomes
- kept presence, roles, visibility, URLs, titles, capabilities, and native
  factory policy consumer-owned
- composed through the existing runtime-generic Tauri window host
- added ordered Surface-flush then window-shutdown receipts
- proved Loophole full hierarchy and Nucleus direct no-Surface shapes
- reassessed Contract 011 as current
- made Card 031 ready

## Package Boundary

| Package | Ordinary dependencies |
| --- | --- |
| `longhorn-surface-windowing` | core, surfaces, windowing |
| `longhorn-tauri-windowing` | core, display, windowing, Tauri |
| `longhorn-nucleus-no-surface-proof` | core, layout, windowing |

`longhorn-tauri-windowing` uses the Surface composition crate only as a test
dependency. The existing host does not acquire Surface authority. The Nucleus
proof has no Surface package in its dependency graph.

## Host Projection

The pure adapter accepts:

- one validated Surface document
- consumer-admitted Surface ids
- current window-placement outcomes
- consumer visibility policy

Only resolved placement outcomes for participating windows establish current
Surface host availability. The adapter runs existing Surface fallback, joins
the result to its placement evidence, and emits canonical bindings plus plain
desired-window inputs. Unavailable and disabled windows remain absent. Direct
window outcomes remain available to a consumer's separate composition path.

## Missing And Returning Hosts

| Current evidence | Resolution | Durable effect |
| --- | --- | --- |
| preferred placement resolved | first preferred host | none |
| preferred unavailable, fallback placement resolved | declared fallback host | none |
| preferred placement returns | first preferred host again | none |
| native host missing, factory unavailable | unsupported create diagnostic | none |
| native host missing, consumer factory enabled | hidden create, place, ready, reveal | none |
| display fallback selected by windowing | current fallback placement retained as evidence | configured home unchanged |

The source Surface document and revision remain exact across every projection,
native failure, and retry.

## Host And Shutdown Evidence

The Tauri mock proof:

1. applies a two-window Surface plan with no dynamic factory and records the
   typed unsupported create diagnostic
2. retries through an injected consumer factory
3. places windows while hidden and reveals only after page readiness
4. preserves protected-primary behavior
5. injects a native position failure and records partial apply evidence
6. retries at a new apply generation and converges
7. flushes Surface persistence before existing host teardown

Surface flush failure blocks teardown for retry. Window teardown failure
retains the completed Surface receipt.

## Composition Proofs

The Loophole-shaped fixture resolves:

```text
WindowId -> SurfaceId -> LayoutContainerId -> RegionId -> PanelId
```

The compile-only Nucleus fixture resolves:

```text
WindowId -> LayoutContainerId -> RegionId -> PanelId
```

Both use the existing layout and window packages. Only the full hierarchy
imports the optional Surface packages.

## Contract Checkpoint

Cards 028-030 did not change Contract 011's transfer binding, revision, or
persistence authority. Surface host fallback remains non-durable, native
window state is not transfer authority, and all transfer commits still require
fresh document resolution. No contract recompilation was needed.

## Validation

- 8 pure Surface/window composition tests pass
- 9 Tauri host-composition tests pass
- the Nucleus no-Surface compile proof passes
- package trees confirm optional dependency direction
- scoped warnings-denied Clippy passes
- no new file crosses the god-file threshold
- Rust 1.85 workspace all-target check passes
- full Effigy QA passes
- Northstar checks pass
- Effigy graph indexing completes with zero diagnostics
- `git diff --check` passes

Effigy doctor remains at the known repository baseline: one generated-in-src
warning and 48 pre-existing god-file warnings.

## Remaining Limits

- no transfer session or drop-zone lease behavior
- no panel or whole-Surface transfer commit
- no generated Surface or transfer TypeScript
- no reusable Svelte/Poodle adapter
- no packaged cross-window drag proof
- no donor migration

These remain assigned to Cards 031-035 and later adoption roadmaps.

## Posture

`strict-ready`

## Next

Start Card 031: implement bounded transfer sessions, complete replacement
drop-zone leases, expiry, cancellation, and deterministic target resolution.
