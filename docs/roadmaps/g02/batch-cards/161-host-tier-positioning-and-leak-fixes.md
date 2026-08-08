# 161 Host Tier Positioning And Leak Fixes

Status: ready
Owner: Tom
Roadmap: g02.012 batch 1
Governing refs: contract 020; research memo 021
Depends on: none
Auto-start next card: no

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
