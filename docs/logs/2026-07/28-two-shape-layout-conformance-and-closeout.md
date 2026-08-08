# Two-shape Layout Conformance And Closeout

Date: 2026-07-28
State: complete implementation batch

## Outcome

- completed Card 027
- closed Cards 023-027 and `g01.005`
- added Rust-generated Loophole and Nucleus conformance fixtures
- used one public default resolver and one shared eight-step mutation runner
- kept Surface and window identities in external fixture-edge bindings
- covered multiple and singleton instance policy
- covered create, close, activate, reorder, move, sizing, and collapse
- covered ordinary empty visibility and transient reveal
- covered exact stale and invalid rejection evidence
- consumed Rust expected snapshots from TypeScript without reimplementing
  mutation
- retained the independent layout/window concurrency proof
- audited Rust and TypeScript package boundaries
- split the definition registry into its own module after binding annotations
  crossed the Effigy hard file-size threshold
- stopped at the `g01.006` planning checkpoint

## Conformance Matrix

| Shape | External host edge | Regions | Sizing slots | Singleton proof | Multiple proof |
| --- | --- | ---: | ---: | --- | --- |
| Loophole | fixture-only `SurfaceId` | 8 | 3 | transport-style panel | workspace tool instances |
| Nucleus | `WindowId` | 5 | 4 | Tasks-style panel | workspace tool instances |

Both shapes:

- resolve the workspace tool default through
  `LayoutDefinitionRegistry::default_region`
- execute the same create/create/activate/reorder/move/size/collapse/close
  sequence through `LayoutMutationEngine`
- finish at revision 8
- retain exactly one moved multiple-instance panel
- reject a second singleton instance
- return exact final state for stale and unchanged-target move rejection
- project visible, hidden, and transiently revealed regions

Checked artifacts:

- `fixtures/layout/loophole-conformance-v1.json`
- `fixtures/layout/nucleus-conformance-v1.json`

## Authority Boundary

The Loophole fixture serializes its Surface binding beside the layout
protocol. The Nucleus fixture does the same for its window binding. Neither
host id appears in definitions, requests, receipts, rejections, or snapshots.

Panel definitions retain layout policy only. Titles, icons, bodies, product
resources, runtime handles, and renderer state remain absent.

The fixtures are evidence, not compatibility adapters. No Loophole, Nucleus,
or Poodle repository changed.

## Persistence And Cross-language Evidence

The existing `independent_layout_and_window_domains_cannot_replace_each_other`
test concurrently publishes a layout mutation and host-owned window mutation
through separate registered descriptors, then reloads both exact values.

TypeScript checks both Rust fixture files. It validates every command, outcome,
and rejection discriminant, exact revision progression, final snapshot
equality, fixed-point sizing, collapse, visibility, singleton rejection, and
JSON round-trip. It does not duplicate the Rust mutation engine.

## Package Report

| Package | Ordinary dependencies |
| --- | --- |
| `longhorn-layout` | `longhorn-core`, serde |
| `longhorn-layout-config` | config, core, layout, serde, serde_json |
| `longhorn-bindings` | core/layout binding features, serde, serde_json, `ts-rs` |
| `@inflatable-cookie/longhorn-layout` | none |

`longhorn-layout` and `longhorn-layout-config` contain no Surface, Tauri,
Svelte, Poodle, or product dependency. `ts-rs` remains feature-gated and
development-only.

## Behavior Delta

| Area | Retained | Intentionally changed | Deferred |
| --- | --- | --- | --- |
| Hosting | Loophole Surface-hosted and Nucleus direct-window shapes | host identity moved outside layout state | Surface lifecycle in `g01.006` |
| Regions | eight-region and five-region semantic compositions | consumer-registered flat schemas replace fixed donor enums | arbitrary split trees remain uncontracted |
| Panels | ordering, active tab, singleton/multiple policy, move and close | product attachments removed from panel instances | panel UI and resource cleanup in consumer/Svelte lanes |
| Mutation | create, close, activate, reorder, move, sizing, collapse | expected revision and atomic authoritative receipts | optimistic renderer state in `g01.007` |
| Sizing | three Loophole and four Nucleus controls | integer-millionth named slots replace renderer floats | Poodle split mapping in `g01.007` |
| Persistence | complete layout snapshots | registered domain and registry-digest migration fail closed | donor storage migration in `g01.014` and `g01.015` |
| Drag | eligible-region reveal | durable placement stays host-authoritative | cross-window sessions and leases in `g01.006` |

## Validation

- Rust conformance unit test passes for both shapes
- 35 layout model tests pass
- 18 layout/config tests include independent-domain concurrency
- 16 Bun protocol/package/conformance tests pass
- deterministic generation and zero-diff check pass
- package dry-run validation passes
- warnings-denied Clippy passes
- Rust 1.85 workspace all-target check passes
- full Effigy QA passes
- Effigy doctor reports no errors; the checked generated source and existing
  large files remain warning-level

## Remaining Limits

- donor repositories are not migrated
- no Surface lifecycle or hosting preference is implemented
- no cross-window transfer session or drop-zone lease is implemented
- no Tauri transport, subscription, Svelte store, or Poodle adapter exists
- the package name remains private pending registry authority

## Posture

`strict-paused`

`g01.005` is complete. `g01.006` requires planning compilation before
implementation.

## Next

Reassess contract 011 and compile strict `g01.006` batch cards. Do not start
Surface or cross-window transfer implementation.
