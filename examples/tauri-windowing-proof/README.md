# Tauri Windowing Proof

Packaged, product-neutral proof for the public `longhorn-tauri-windowing` host.
It uses static HTML and an injected JSON placement sink. It has no layout,
Surface, Poodle, or Longhorn configuration dependency.

Build the macOS application:

```sh
effigy proof-windowing-build
```

The app writes structured evidence and staged placement data beneath its Tauri
application-data directory. The exact paths are shown in the operator panel.

The main window is predeclared and protected. The `workspace` window is created
and closed dynamically. Use the restart controls to exercise maximized restore
and the explicit missing-saved-display fallback.
