# Longhorn — current state

Longhorn is a pre-1.0 workspace of shared Rust and Svelte/TypeScript systems
for Tauri desktop apps, with GPUI as a second first-class host. Five consumers
are migrated: Nucleus, Loophole, Soundcheck, Split-shell and Jetstream.

- **Released:** `0.3.4` (2026-10-21). Three npm packages under
  `@inflatable-cookie` and git tag `v0.3.4` for the Rust crates. The `0.3.x`
  line adds desktop support crates extracted from Acowtancy Desktop:
  `longhorn-redact` in `0.3.1`
  ([contract 024](knowledge/contracts/024-text-redaction.md)),
  `longhorn-support-log` in `0.3.2`
  ([contract 025](knowledge/contracts/025-bounded-support-logs.md)),
  `longhorn-diagnostics` in `0.3.3`
  ([contract 026](knowledge/contracts/026-diagnostics-bundle.md)), and
  `longhorn-tool-provision` with `longhorn-archive-path`
  ([contract 027](knowledge/contracts/027-managed-tool-provisioning.md)) and
  `longhorn-pty` ([contract 028](knowledge/contracts/028-pty-sessions.md)) in
  `0.3.4`. `0.3.4` also fixes `evaluate` scripts that begin with `(` or `[`.
  `0.3.0` set the Svelte floor `5.56.8` and removed the `dev` feature on
  `longhorn-tauri-agent-control`. Poodle is a peer range (`>=0.4.4 <0.5`)
  and third-party Rust dependencies are ranges
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
