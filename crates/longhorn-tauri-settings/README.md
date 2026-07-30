# longhorn-tauri-settings

Narrow Tauri command and event adapter for an injected settings registry and
authority. The crate owns no application configuration, page schema, renderer,
or global singleton.

Consumers install `TauriSettingsState` and the four exported commands, then
choose the read-only or mutable capability example. The injected
`SettingsAuthority` remains responsible for caller authorization and product
semantics.

The commands are `longhorn_settings_registry`, `longhorn_settings_load`,
`longhorn_settings_apply`, and `longhorn_settings_reset`. Registry and scope
events use `longhorn://settings/registry-changed` and
`longhorn://settings/scope-changed`; they carry revision hints, never mutable
authority.
