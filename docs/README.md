# Longhorn — current state

Longhorn is a pre-1.0 workspace of shared Rust and Svelte/TypeScript systems
for Tauri desktop apps, with GPUI as a second first-class host. Five consumers
are migrated: Nucleus, Loophole, Soundcheck, Split-shell and Jetstream.

- **Released:** `0.3.0` (2026-09-27). Three npm packages under
  `@inflatable-cookie` and git tag `v0.3.0` for the Rust crates. Breaking:
  Svelte floor `5.56.8`, and no `dev` feature on
  `longhorn-tauri-agent-control`. Poodle is a peer range (`>=0.4.4 <0.5`) and
  third-party Rust dependencies are ranges
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
