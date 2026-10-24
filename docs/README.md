# Longhorn — current state

Longhorn is a pre-1.0 workspace of shared Rust and Svelte/TypeScript systems
for Tauri desktop apps, with GPUI as a second first-class host. Four consumers are migrated: Nucleus, Loophole, Soundcheck and Jetstream. A minimal consumer was retired on 2026-10-22 ([Q-006](knowledge/questions.md)).

- **Released:** `0.4.3` (2026-10-24). Three npm packages under
  `@inflatable-cookie` and git tag `v0.4.3` for the Rust crates. `0.4.3`
  fixes PTY close and resize for exited process groups
  ([contract 028](knowledge/contracts/028-pty-sessions.md)) and updates Tokio to
  `1.53.3` in the workspace and affected prototype locks for
  `RUSTSEC-2026-0386`. Git-tag consumers still need a patched Tokio in their
  own lockfiles ([release contract](knowledge/contracts/release.md#lock-maintenance)). `0.4.2`
  adds `longhorn-git-workspace`
  ([contract 029](knowledge/contracts/029-git-workspace.md)): Git workspace
  execution, status, sync, clone and attach, plus GitHub device-flow sign-in
  and an askpass credential bridge. `0.4.1`
  adds per-root storage resolution (`resolve_storage_root`) and one native
  directory fact on demand (`native_platform_directory_fact`), so a CLI
  needs only the inputs of the roots it uses. `0.4.0` was breaking:
  storage identity, layout resolution, `StorageRoots` and the bootstrap
  locator moved from `longhorn-config` to `longhorn-storage-layout`
  ([contract 004](knowledge/contracts/004-configuration-storage-backup-and-recovery.md)).
  Only the import paths change; the paths on disk do not. `0.4.0` also adds a
  native platform-directory facts provider for hosts without Tauri, and
  read-only human-written TOML domains in `longhorn-config` (feature `toml`),
  so CLI apps can adopt Longhorn's storage layout (guide: "Longhorn For CLI
  Apps" in [storage, configuration and backup](guides/storage-configuration-backup.md)).
  The `0.3.x` line added the desktop support crates extracted from Acowtancy
  Desktop: `longhorn-redact`
  ([contract 024](knowledge/contracts/024-text-redaction.md)),
  `longhorn-support-log`
  ([contract 025](knowledge/contracts/025-bounded-support-logs.md)),
  `longhorn-diagnostics`
  ([contract 026](knowledge/contracts/026-diagnostics-bundle.md)),
  `longhorn-tool-provision` with `longhorn-archive-path`
  ([contract 027](knowledge/contracts/027-managed-tool-provisioning.md)) and
  `longhorn-pty` ([contract 028](knowledge/contracts/028-pty-sessions.md)).
  Poodle is a peer range (`>=0.4.4 <0.5`) and third-party Rust dependencies
  are ranges
  ([contract 012](knowledge/contracts/012-distribution-and-compatibility.md)).
- **Held surface:** built but not consumer-ready; see the
  [register](reference/held-surface.md).
- **Consumer adoption** of Longhorn features is consumer-owned. Longhorn does
  not write to consumer repositories without the operator's go-ahead.

## By topic

- Vision: [knowledge/vision.md](knowledge/vision.md)
- Architecture: [knowledge/architecture/](knowledge/architecture/README.md)
- Contracts: [knowledge/contracts/](knowledge/contracts/contract-index.md)
- Release: [knowledge/contracts/release.md](knowledge/contracts/release.md)
- All knowledge: [knowledge/README.md](knowledge/README.md)

For consumers:

- [Adoption guides](guides/README.md) and [glossary](guides/glossary.md)
- [API reference](reference/README.md)

## What's next

The project's plan is in Queue: its lanes, their documents and their order.
