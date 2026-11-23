### Changed

- Unified the Tauri adapter policy across all `longhorn-tauri-*` crates
  (contracts 010 and 020):
  - **One authority trait per adapter:** removed mirroring `*Service` traits
    (`BridgeCommandService`, `CommandHostService`,
    `ConfigOperationsCommandService`, `ForkHistoryHostService`,
    `HistoryHostService`, `LicenceHostService`, `NotificationHostService`,
    `OperationHostService`, `SettingsCommandService`, `UpdateHostService`,
    `TransferCommandService`, `SurfaceTransferCommandService`). Standardized
    authority naming to `<Domain>HostAuthority` (`ConfigOperationsAuthority` ->
    `ConfigOperationsHostAuthority`, `SettingsAuthority` ->
    `SettingsHostAuthority`). `Tauri*State::new` now takes the handler assembly
    directly (`Arc<*HandlerAssembly<dyn *HostAuthority>>`, or
    `Arc<OperationHandlerAssembly>`, `Arc<AssembledTransferCommands<...>>`).
  - **Off-main-thread commands:** converted sync `#[tauri::command]` handlers in
    `longhorn-tauri-bridge`, `longhorn-tauri-history`,
    `longhorn-tauri-history-tree`, `longhorn-tauri-licence`,
    `longhorn-tauri-notifications`, `longhorn-tauri-operation`,
    `longhorn-tauri-transfer`, and `longhorn-tauri-update` to `async` using
    `tauri::async_runtime::spawn_blocking`.
  - **Non-retryable poison:** poisoned mutexes and `spawn_blocking` join
    failures now report `retryable: false` across all `longhorn-tauri-*`
    adapters; `longhorn-core::MemoryCredentialStore` returns
    `CredentialError::Unavailable` on poison instead of recovering with
    `into_inner`; `BridgeHandlerAssembly::teardown` returns
    `Result<(), BridgeHostError>` instead of silently skipping on poison.
  - **Typed wire errors in `longhorn-tauri-transfer`:** replaced
    `Result<_, String>` command returns and `{code:?}` formatting with typed
    `TransferHostError` / `TransferHostErrorCode`; removed unused
    `TransferHandlerError::Projection`; made
    `TransferHandlerAssembly::project_selector` `pub(crate)`; removed the shadow
    client map in favor of `TransferCoordinator::current_client`; released the
    coordinator mutex before calling `PanelTransferAdapter` /
    `SurfaceTransferAdapter` and replaced raw `&mut TransferCoordinator` with
    `TransferSessionRegistrar` (start) and pre-resolved target attempts (commit).
  - **`longhorn-tauri-bridge`:** added `bridge_plugin` to install
    `TauriBridgeState` and wire window-destroyed teardown by default; collapsed
    duplicate `authorize_*` helpers.
  - **`longhorn-tauri-settings`:** wired optional
    `ConsumerSettingsTransactionAuthority` support via
    `SettingsHandlerAssembly::with_transaction_authority` and
    `SettingsHostAuthority::authorize_transaction`.
  - **`longhorn-tauri-notifications`:** scoped command-triggered change hints to
    the invoking `WebviewWindow` (`window.emit`) instead of app-wide
    `window.app_handle().emit`.
  - **`longhorn-tauri-operation`:** derived the poisoned-executor dispatch code
    from `OperationHostErrorCode::ExecutorStateUnavailable::as_str()` with
    `retryable: false` instead of a string literal with `retryable: true`.

### Breaking Changes

- **Consumer migration (`acowtancy`):**
  `apps/bovine-desktop/src-tauri/src/config_operations.rs` previously imported
  and implemented `longhorn_tauri_config::ConfigOperationsCommandService`.
  Migrate by:
  1. Replace `ConfigOperationsCommandService` with
     `ConfigOperationsHostAuthority` and change method receivers from `&self` to
     `&mut self` (or wrap the inner state in
     `ConfigOperationsHandlerAssembly::new(authority)`).
  2. Pass `Arc::new(ConfigOperationsHandlerAssembly::new(authority))` to
     `TauriConfigOperationsState::new(...)`.
  3. Update any callers of `ConfigOperationsAuthority` or `SettingsAuthority` to
     `ConfigOperationsHostAuthority` / `SettingsHostAuthority`.
- `PanelTransferAdapter` and `SurfaceTransferAdapter` no longer receive
  `&mut TransferCoordinator`; use `TransferSessionRegistrar::create_session` in
  `start_*` (with `prepare_panel_session_source` /
  `prepare_surface_session_source`) and accept `TerminalTransferAttempt` /
  `TerminalTransferResolution` in `commit_*` (with
  `commit_resolved_panel_transfer` / `commit_resolved_surface_transfer`).
