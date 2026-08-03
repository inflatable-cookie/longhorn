# Child-view Navigation Authority

Date: 2026-08-03
Card: 132
Roadmap: g01.020

## Result

Figmatic g03.006 exposed a real child-view mechanism gap. The adapter retained
one opaque Tauri webview but offered no canonical way to change its document.
Nucleus worked around the same gap through global Tauri label lookup.

Contract 017 now assigns navigation execution to the child-view mechanism
while URL targets and admission policy remain consumer-owned. Navigation does
not enter `NativeContentOperation`, `@longhorn/native-content`, or the common
desired/observed state.

## Public Semantics

- exact attach generation is checked before policy or native work
- the existing `ChildViewSpec` policy admits the parsed URL
- fresh current URL is observed through the retained private handle
- a current URL returns `unchanged` without reload
- a different URL submits exactly one native navigation
- the receipt records previous and requested URLs but never claims load
  completion
- page-load start clears readiness; page-load finish restores it
- denial, URL observation failure, navigation failure, stale/future authority,
  and absent/retired attachment are typed

## Boundary

Back, forward, reload, history persistence, redirects, permissions, downloads,
popups, selection identity, and renderer authorization remain downstream. A
consumer command may call the adapter under its own authorization. Raw handle
or global-label lookup is not the canonical path.

## Validation

- `effigy qa:native-content-child-view`
- `effigy check:native-content-child-view-graph`
- `effigy check:native-content-bindings`

## Next

Card 133 extends the real packaged macOS proof with same-origin navigation,
same-URL idempotence, denial, readiness, and retained-generation evidence.
