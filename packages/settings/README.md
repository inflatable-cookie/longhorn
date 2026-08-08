# @inflatable-cookie/longhorn-settings

Framework-neutral checked protocol and client for the Longhorn settings
authority. Rust serde types are authoritative.

```sh
effigy generate:settings
effigy test:settings-ts
effigy test:settings-svelte
```

The root exports generated types, compatibility guards, deterministic registry
projection/search/deep-link helpers, and listener-first clients. It imports no
browser global, Svelte, Poodle, layout, Surface, command, history, or backend
package. Consumer codecs retain ownership of opaque values and page renderers.

Optional entry points:

- `@inflatable-cookie/longhorn-settings/svelte` provides per-instance registry, scope, route,
  draft, mutation, conflict, recovery, activation, guard, and teardown state.
- `@inflatable-cookie/longhorn-settings/poodle` provides one public-Poodle shell over that
  controller for modal, independent-window, and routed-panel hosts.

Consumer renderer resolvers return Svelte snippets keyed by the sealed
registry. Page snippets keep product form state, validation, copy, and intent
codecs. A staged page-level Apply is available only for one dirty failure-atomic
unit; broader transaction authority remains consumer-owned.

Transport names are explicit:

- `longhorn_settings_registry`
- `longhorn_settings_load`
- `longhorn_settings_apply`
- `longhorn_settings_reset`
- `longhorn://settings/registry-changed`
- `longhorn://settings/scope-changed`
