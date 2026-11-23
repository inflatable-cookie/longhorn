# Window-Bound Hosting Proof

This compile-only example binds panels directly to a window with one unlabelled
layout Surface from `longhorn-surfaces`. The transfer dependency exercises the
direct-window panel-host binding.

```text
WindowId -> SurfaceId -> RegionId -> PanelInstanceId
```

```sh
oxide check -p longhorn-window-bound-hosting-proof
oxide tree -p longhorn-window-bound-hosting-proof --edges normal
```
