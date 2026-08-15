# Nucleus No-Surface Proof

This compile-only example proves the no-Surface-feature hosting shape:

```text
WindowId -> SurfaceId -> RegionId -> PanelId
```

A Surface is the layout since Card 179, so this host composes one unlabelled
Surface through `longhorn-surfaces` and never uses a Surface feature. The
name predates the absorption; what it asserts is the absence of the Surface
*feature*, not of the crate. It also depends on `longhorn-core`,
`longhorn-transfer`, and `longhorn-windowing`. The transfer dependency
exercises the direct-window panel-host binding.

Run:

```sh
cargo check -p longhorn-nucleus-no-surface-proof
cargo tree -p longhorn-nucleus-no-surface-proof --edges normal
```
