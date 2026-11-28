# Shared Desktop System Suite

This map groups reusable capabilities by what an application needs. The
[contract index](../contracts/contract-index.md) defines which rules apply to
the shared core, each host, and the optional webview edge. The
[API surface](../../reference/api-surface.md) lists packages and features.

| Capability | Owning boundary |
| --- | --- |
| Display identity, geometry and window planning | [009](../contracts/009-display-identity-coordinates-and-window-planning.md) |
| Window lifecycle and native execution for Tauri and GPUI | [020](../contracts/020-host-adapter-boundary.md) |
| Optional layout and Surface hosting | [002](../contracts/002-composable-workspace-hosting.md) |
| Cross-window panel and Surface transfer | [011](../contracts/011-cross-window-transfer.md) |
| Storage roots, domain state, backups and recovery | [004](../contracts/004-configuration-storage-backup-and-recovery.md) |
| Settings registry, apply units and shell | [005](../contracts/005-settings-and-system-registration.md) |
| Command registry, physical input, keymaps and palette projection | [006](../contracts/006-command-action-and-input.md) |
| Optional service topology and supervision | [007](../contracts/007-optional-backend-topology.md) |
| Checked webview IPC and events | [010](../contracts/010-rust-typescript-ipc-and-events.md) |
| Svelte lifetime and public Poodle bindings | [013](../contracts/013-svelte-and-poodle-adapter-lifecycle.md) |
| Linear history and optional fork-tree history | [008](../contracts/008-history-kernel-boundary.md) |
| Finite operations and cancellation | [015](../contracts/015-async-operation-lifecycle.md) |
| Retained notifications and transient projection | [016](../contracts/016-notification-ledger-and-projection.md) |
| Child-webview, isolated-window and backing-surface coordination | [017](../contracts/017-native-content-island-coordination.md) |
| Staged app updates, release channels and restart admission | [018](../contracts/018-application-update-and-release-channels.md) |
| Licensing, entitlement, activation and leases | [019](../contracts/019-licensing-entitlement-and-activation.md) |
| Consumer-scoped credential references | [021](../contracts/021-consumer-scoped-credential-slots.md) |
| Opt-in agent app control | [022](../contracts/022-agent-app-control.md) |
| Transport-neutral contextual tool dispatch and validation | [023](../contracts/023-production-contextual-agent-tool-boundary.md) |
| Secret redaction | [024](../contracts/024-text-redaction.md) |
| Bounded support logs and output tails | [025](../contracts/025-bounded-support-logs.md) |
| Diagnostics bundle rendering | [026](../contracts/026-diagnostics-bundle.md) |
| Verified tool provisioning and discovery | [027](../contracts/027-managed-tool-provisioning.md) |
| PTY process and session lifecycle | [028](../contracts/028-pty-sessions.md) |
| Policy-hooked Git workspace operations and credentials | [029](../contracts/029-git-workspace.md) |

An app can omit layout. If it uses registered regions and panels, the Surface
owns the layout and every host uses the binding chain in
[contract 002](../contracts/002-composable-workspace-hosting.md#hosting-composition).
Contract 014 is superseded there.

Lifecycle and app chrome beyond host window events remain consumer
composition. The [greenfield examples](../../../examples/greenfield-compositions/README.md)
show supported starting shapes. The
[held-surface register](../../reference/held-surface.md) owns selection limits
for implemented capabilities awaiting a consumer or product decision.
