# Nucleus No-Surface Proof

This compile-only example proves the direct hosting shape:

```text
WindowId -> LayoutContainerId -> RegionId -> PanelId
```

It depends on `longhorn-core`, `longhorn-layout`, `longhorn-transfer`, and
`longhorn-windowing`. The transfer dependency exercises the direct-window
panel-host binding.
It does not depend on `longhorn-surfaces` or
`longhorn-surface-windowing`.

Run:

```sh
cargo check -p longhorn-nucleus-no-surface-proof
cargo tree -p longhorn-nucleus-no-surface-proof --edges normal
```
