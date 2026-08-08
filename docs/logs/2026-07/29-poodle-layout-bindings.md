# Poodle Layout Bindings

Date: 2026-07-29
Card: 039
State: complete

## Outcome

- added private, Surface-free `@inflatable-cookie/longhorn-poodle`
- bound authoritative layout state to public Poodle `Tabs`, `DockRegion`, and
  `SplitView`
- kept labels, icons, bodies, and static panel rendering consumer-owned
- dispatched activate, close, reorder, move, collapse, and sizing commands
  with generated protocol types
- serialized UI mutations so each request uses the latest reconciled revision
- mounted the five-region Nucleus and eight-region Loophole policy shapes
- verified the exact Card 038 Poodle preview artifact before checks

## Boundary

Longhorn owns registered layout policy, ids, controlled values, eligibility,
request ids, expected revisions, optimistic projection, and reconciliation.
Poodle owns markup, interaction semantics, accessibility, local drag
presentation, and visual affordances. Consumers supply product labels, icons,
and Svelte snippets.

The package imports no Surface or transfer capability. Cross-window drag
remains Card 040 work.

## Longhorn To Poodle Map

| Public component | Longhorn projection | Poodle input | Poodle event | Authoritative command |
| --- | --- | --- | --- | --- |
| `Tabs` | ordered panel instances | `items`, `value`, closeable metadata | `onValueChange` | `activate_panel` |
| `Tabs` | registered region order | `reorderable` | `onReorder` | `reorder_region` |
| `Tabs` | panel policy | item `closable` | `onClose` | `close_panel` |
| `DockRegion` | region collapse and placement policy | `collapsed`, `collapsible`, `canAcceptPanel` | `onCollapsedChange` | `set_region_collapsed` |
| `DockRegion` | target container and region | public panel-drop callback | `onPanelDrop` | `move_panel` |
| `DockRegion` | ordered panel instances | `items`, `value` | value, close, reorder events | activate, close, reorder |
| `SplitView` | sizing definition and current millionth ratio | `ratio`, `minRatio`, `maxRatio` | `onRatioChange` | `set_sizing_slot` |
| `SplitView` | collapsible region state | primary/secondary collapsed props | collapse callbacks | `set_region_collapsed` |

`PanelPresentationResolver` supplies labels and optional public `IconProp`
values. `LayoutTabs.body`, `LayoutDockRegion.body`, and
`LayoutDockRegion.panel` receive `PanelRenderContext` snippets. Product
metadata never enters the durable layout document.

## Controlled-state Matrix

| Mutation | Optimistic projection | Reconciliation evidence |
| --- | --- | --- |
| activate | active panel changes | rejection restores authoritative selection |
| close | member is removed; adjacent/end fallback becomes active | authoritative result settles by request id |
| reorder | complete region order changes | authoritative result replaces projection |
| move | source removal and target insertion happen together | registered eligibility precedes dispatch |
| collapse | supported boolean changes | rejection restores authoritative posture |
| sizing | fixed-point slot ratio changes | authoritative result replaces projection |

Back-to-back commands run in binding order. Request construction happens only
after the preceding result settles, so `expected_revision` advances from the
returned authority instead of reusing a stale optimistic revision. Dispatch
errors report through the injected handler without poisoning later work.

## Mounted Proof

- `tabs.test.ts`: immediate selection, rejection rollback, and close fallback
- `dock.test.ts`: consumer static-panel snippet, same-region reorder, and move
  through Poodle's public panel-drop callback
- `split.test.ts`: collapse projection, rejection rollback, and sizing command
- `binding.test.ts`: explicit missing presentation, invalid collapse mapping,
  and serialized revision advancement
- `shapes.test.ts`: five mounted Nucleus regions and eight mounted Loophole
  regions from the shared conformance fixtures
- `packages/svelte/tests/layout.test.ts`: stale completion cannot replace newer
  authority

The Nucleus fixture imports layout and the Surface-free Svelte layout subpath
only. Package tests reject Surface, transfer, private selector, private MIME,
and donor source-path knowledge.

## Exact Preview Artifact

Set:
`39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`

Evidence:
`~/Dev/projects/poodle/.artifacts/g12.016-A698XB/evidence.json`

`verify:poodle-preview` checks the Svelte range `>=5.38.6 <6`, recomputes all
five tarball SHA-256 digests, and recomputes the set membership id. The root
workspace installs those exact tarballs. `@inflatable-cookie/longhorn-poodle` declares exact
`0.1.0` Longhorn dependencies and exact `@inflatable-cookie/poodle-svelte@0.1.0` peer
compatibility; it makes no broader Poodle compatibility claim.

The dry-run package contains ten files and unpacks to 27.87 KB. The installed
Poodle root resolves outside the donor source tree.

## Validation

- `effigy verify:poodle-preview`
- `effigy check:poodle-ts`
- `effigy check:poodle-svelte`: zero errors and warnings
- `effigy test:poodle`: 6 files, 13 tests
- `effigy check:poodle-package`
- all `effigy qa` constituent tasks passed; macOS stalled twice in `dyld`
  before the linker-signed `longhorn-bindings` executable entered user code,
  so that disposable artifact was locally re-signed and the generated-binding
  and remaining QA tasks were rerun directly
- `effigy scan god-files`: unchanged single high finding in
  `longhorn-tauri-windowing/src/lifecycle/model.rs`; no new high finding

## Current State

Card 039 is complete. Card 040 is ready and has not started.

## Next

Start Card 040 cross-window drag and titlebar actions.
