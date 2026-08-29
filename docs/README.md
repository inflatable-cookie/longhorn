# Longhorn Docs

The documentation for Longhorn, a pre-1.0 private workspace of shared Rust
and Svelte/TypeScript systems for Tauri desktop apps. Package-manager
publication is deferred; this repo records both how the systems work and how
they were built.

## Start Here

For new readers, in order:

1. [Vision](vision/README.md) — what Longhorn is and why it exists
2. [Adoption Guides](guides/README.md) — how to choose and integrate packages
3. [Glossary](guides/glossary.md) — the terms the docs use
4. [API Reference](reference/README.md) — exact packages, crates, entrypoints
5. [Architecture](architecture/README.md) — how the systems are structured
6. [Contracts](contracts/README.md) — the rules the systems must follow

For maintainers and agents:

- [Research](research/README.md) — the audits and boundary memos behind the design
- [Specs](specs/README.md) — provisional planning before promotion
- [Roadmaps](roadmaps/README.md) — delivery planning, including the
  [generation index](roadmaps/generation-index.md) with the live next-task pointer
- [Logs](logs/README.md) — batch evidence of completed work
- [Triage](triage/README.md) — open observations awaiting promotion or disposition

## Current State

Pre-1.0 private workspace. Generation g01 is complete (20 milestones, cards
001-137, five migrated consumers). Generation g02 is active across
application-update and licensing runways, the memo-023 release-audit suite
(cards 200-224), the mixed-scale coordinate correction (card 226), and the
agent-control arc through g02.035 (cards 227-240), with consumer-scoped
credential slots at card 225. The private `0.1.0` compatibility candidate
spans the TypeScript packages and Rust crates enumerated — always currently —
by the generated [API reference](reference/api-surface.md). Publication to
npm and crates.io remains deferred.

The complete delivery history is preserved in the
[roadmap generation index](roadmaps/generation-index.md).
