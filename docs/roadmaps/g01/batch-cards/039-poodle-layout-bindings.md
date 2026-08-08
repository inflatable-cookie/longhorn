# 039 Poodle Layout Bindings

Status: complete (2026-07-29)
Owner: Tom
Roadmap: g01.007 batch 2
Governing refs: contracts 012-014; research memos 009 and 011
Depends on: Cards 037 and 038
Auto-start next card: no

## Objective

Bind authoritative layout projection and dispatch to public Poodle Tabs,
DockRegion, and SplitView APIs while keeping panel presentation consumer-owned.

## Scope

- private `@inflatable-cookie/longhorn-poodle` package
- exact Card 038 Poodle preview artifact set
- Surface-free root layout bindings
- tab selection, close, reorder, and body snippets
- dock selection, collapse, eligibility, move, reorder, and panel snippets
- split ratio and collapse bindings
- consumer label, icon, and panel-body resolvers
- mounted controlled-state and package checks

## Public Behavior

Longhorn projects ids, current values, ratios, collapse state, eligibility, and
typed dispatch. Poodle renders and owns interaction semantics. Consumers
resolve labels, icons, empty states, panel bodies, and product presentation.

Same-window drop and reorder dispatch the existing authoritative layout
mutation. Poodle's workspace presentation snapshot never becomes a durable
Longhorn document.

## Out Of Scope

- cross-window transfer sessions
- Surface tabs
- custom menu, dialog, or shell components
- Poodle source copies
- product panel registries
- consumer migration

## Steps

1. Install and verify Card 038 artifact set
   `39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`.
2. Add the private package with exact peers and Surface-free exports.
3. Project registered Longhorn regions and panel instances into public tab
   items.
4. Bind tab activate, close, and reorder to typed requests.
5. Bind DockRegion collapse, eligibility, move, reorder, and snippets.
6. Bind SplitView ratio and collapsed posture to sizing-slot requests.
7. Accept consumer metadata and body resolvers without persisting them.
8. Reconcile each controlled component to authoritative response state.
9. Add minimal Nucleus-shaped and full region-shaped mounted fixtures.
10. Check imports, package contents, and absence of Poodle internals.

## Acceptance Criteria

- only public Poodle exports, props, snippets, and events are used
- no Poodle component source is copied or patched in Longhorn
- every mutation carries request id and expected revision
- rejection restores authoritative component state
- missing product metadata remains an explicit consumer condition
- panel bodies remain snippets
- Nucleus fixture imports no Surface package
- root package resolves no Surface or transfer code
- exact packed Poodle preview passes mounted checks
- the installed artifact set id and five tarball digests match Card 038

## Evidence Required

- Longhorn-to-Poodle prop and event map
- controlled-state mutation matrix
- rejection and stale-response fixtures
- Nucleus-shaped and full-region mounted snapshots
- public-import audit
- package and dependency reports
- Svelte check, TypeScript, and Effigy QA

## Stop Conditions

- a needed Poodle behavior is not public
- binding requires private DOM or payload knowledge
- Poodle state must become persistence authority
- one wrapper only renames props without owning useful integration
- product metadata must enter Longhorn

## Result

Private `@inflatable-cookie/longhorn-poodle` now binds authoritative layout projection and
revisioned mutation to public Poodle Tabs, DockRegion, and SplitView APIs. It
is Surface-free, keeps product presentation in consumer resolvers and
snippets, serializes requests against reconciled authority, and mounts both
Nucleus and Loophole policy shapes against the exact Card 038 artifact.

Evidence:
`../../../logs/2026-07/29-poodle-layout-bindings.md`.

## Next Task

Card 040 is ready but not started. It adds armed cross-window transfer actions,
compatible-region reveal, and shared titlebar chrome.
