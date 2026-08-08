# 161 Host Tier Positioning And Leak Fixes

Status: complete
Owner: Tom
Roadmap: g02.012 batch 1
Governing refs: contract 020; research memo 021
Depends on: none
Auto-start next card: no
Completed: 2026-08-08

## Objective

Make the architecture honest about what it already is: a host-agnostic core
with pluggable backends. Restate positioning, tier the contracts, and close
the two leaks measurement found.

## Scope

- front doors and README positioning
- contract tiering (already recorded in `contract-index.md`; propagate it)
- move four pure types out of `longhorn-tauri-windowing`
- generalise the webview-shaped geometry in `longhorn-core`

## Steps

1. Restate positioning wherever the docs say "Tauri applications". Longhorn
   is a Rust desktop application framework with pluggable host backends, of
   which Tauri is one of two first-class targets.
2. Move `CapturedWindowPlacement`, `WindowFlushRequest`,
   `WindowPlacementFlushTicket` and `WindowPlacementSink` from
   `longhorn-tauri-windowing` to `longhorn-windowing`. All four are already
   Tauri-free — `WindowPlacementSink` is a plain trait,
   `WindowPlacementFlushTicket` wraps a channel receiver. This removes the
   only pure-to-host dependency edge in the workspace.
3. Generalise `ClientCssPx` and the client geometry types in
   `longhorn-core`. CSS pixels are a webview concept; GPUI has logical and
   physical pixels with a scale factor. Pick a host-neutral name and record
   the mapping for each backend.
4. Audit the remaining doc-comment references to Tauri in pure crates.
   `longhorn-windowing`'s "such as a Tauri window label" is fine — the type
   is opaque and the comment gives an example. `BridgeHostForm::TauriLocal`
   is fine — it names a real host form.
5. Propagate the tiering into `docs/README.md` and the architecture front
   door so a reader can tell which contracts apply to their backend.

## Progress — 2026-08-08

**The pure-to-host edge is gone.** A workspace scan now reports zero pure
crates depending on any `longhorn-tauri-*` crate.

Moved into `longhorn-windowing`: `lifecycle/capture.rs` and
`lifecycle/flush.rs` wholesale (both were already Tauri-free), plus the
placement port extracted from `services/ports.rs` into a new
`lifecycle/placement_sink.rs`.

**Consumers did not break.** `longhorn-tauri-windowing` re-exports the moved
types at their previous paths, so its public surface is unchanged. That is a
facade, not a compatibility shim: the point was to correct the dependency
direction, and four consumer files import these types
(`nucleus/apps/desktop`, `loophole/aura`, `soundcheck`, `finch`). All four
compile unchanged.

`ScheduledWindowLifecycleWake::new` was `pub(crate)` and is now `pub` — host
adapters construct wakes, and the type is host-agnostic now.

### Finding — the ports module is already the host contract

Only **two** of eleven types in `services/ports.rs` are Tauri-bound:
`WindowCaptureBackend<R: Runtime>` and `WindowRevealBackend<R: Runtime>`
plus its `TauriWindowRevealBackend` impl. The other nine are pure, and they
map directly onto contract 020's requirements — `WindowUserCloseHandler` is
close handling, `WindowLifecycleReporter` is lifecycle events,
`WindowPlacementSink` is placement application.

The host seam already exists in Rust; it was written inside the Tauri crate.
Only the placement port moved in this card, because that is what removed the
dependency edge. Whether the remaining seven pure port types should also
move is a contract-020 question that **Card 163 should answer with evidence**
rather than this card answering by assumption.

### Geometry generalisation — and a correction

This was recorded as a breaking change touching every consumer's public
surface. Measurement says otherwise, per type:

| type | longhorn refs | consumer files |
| --- | ---: | ---: |
| `ClientCssPx` | 27 | **0** |
| `ClientPoint` | 51 | 4 |
| `ClientSize` | 66 | 4 |
| `ClientRect` | 67 | 4 |
| `ClientGeometryError` | 12 | 0 |

The only badly-named type is the one nothing outside Longhorn references.
`ClientPoint`, `ClientSize` and `ClientRect` are already host-neutral names;
only their doc comments said "webview".

`ClientCssPx` is now `ClientLogicalPx`, and the doc comments are host-neutral.
The concepts were never different: a webview calls these CSS pixels and GPUI
calls them logical pixels, and both mean a device-independent unit scaled to
physical by the display's scale factor. This was naming, not modelling.

The generated TypeScript alias renames with it, but `ClientPoint` and
`ClientSize` keep their names and structural shape (`{ x: number, y: number }`),
so no consumer breaks. All four consumer repositories compile unchanged.

## Acceptance Criteria

- `longhorn-windowing-config` compiles with no `longhorn-tauri-*` dependency
- no host-shaped concept sits unqualified in `longhorn-core`
- the docs state the tier of every contract
- no behaviour change; workspace QA passes
- bindings regenerate with no semantic diff

## Evidence Required

- the dependency-edge scan showing zero pure-to-host edges
- the geometry rename with its per-backend mapping recorded

## Stop Conditions

- the geometry generalisation cannot be expressed without a breaking change
  to a consumer's public surface, in which case it is scheduled against the
  next distribution candidate rather than taken silently

## Next Task

Cards 162 and 163, which are independent of each other.
