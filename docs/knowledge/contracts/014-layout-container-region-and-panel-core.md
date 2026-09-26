# 014 Layout Container Region And Panel Core

Status: superseded by [contract 002](002-composable-workspace-hosting.md) —
2026-08-11

## Superseded

Card 179 removed the layout container. A Surface now carries the schema it
instantiates, its regions, its sizing slots and the panel instances placed in
them, so the separate layout document this contract governed no longer exists
and `LayoutContainerId` is gone from the model.

Everything this contract said that still holds moved into contract 002: the
definition registry, layout state, normalization, region visibility, the layout
mutation protocol, persistence, and the Rust and TypeScript boundary.

One section did not move. This contract's Composition section stated two
binding chains, one for hosts with Surfaces and one for hosts without:

```text
WindowId -> LayoutContainerId -> RegionId -> PanelInstanceId
WindowId -> SurfaceId -> LayoutContainerId -> RegionId -> PanelInstanceId
```

That pair *was* the container abstraction written down, and it collapses to
one chain. A host that wants no Surface concept uses a single unlabelled
Surface, which is what the no-Surface conformance shape had been doing all
along under a different name:

```text
WindowId -> SurfaceId -> RegionId -> PanelInstanceId
```

## Why This File Remains

Twelve documents reference this contract, including two research translation
memos and a completed g01 card. They record what governed that work at the
time, and repointing them at 002 would falsify it. The file stays so every
inbound link resolves and the history stays honest about what this contract
once required.

This is the first retired contract of twenty, so it sets the shape: keep the
file, replace the body with a pointer and a statement of what absorbed it and
when, and leave every inbound link working.
