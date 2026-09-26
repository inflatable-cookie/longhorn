# Longhorn — current state

Longhorn is a pre-1.0 workspace of shared Rust and Svelte/TypeScript systems
for Tauri desktop apps, with GPUI as a second first-class host. Five consumers
are migrated: Nucleus, Loophole, Soundcheck, Split-shell and Jetstream.

- **Released:** `0.2.1` (2026-09-26). Three npm packages under
  `@inflatable-cookie` and git tag `v0.2.1` for the Rust crates. Figmatic
  consumes the exact pins.
- **On `main`, not yet released:** Rust-side agent file selection and the
  `set_file_input` tool (both under [contract 022](knowledge/contracts/022-agent-app-control.md)),
  and the `effigy release:bump` command.
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

See [plan.md](plan.md). Unresolved leads live in [triage/](triage/README.md).
