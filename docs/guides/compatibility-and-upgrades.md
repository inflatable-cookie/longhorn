# Compatibility And Upgrades

Status: checked private adoption guidance
Updated: 2026-08-02
Governing contracts: [010](../contracts/010-rust-typescript-ipc-and-events.md)
and [012](../contracts/012-distribution-and-compatibility.md)

## Availability

Longhorn is a coordinated private `0.1.0` workspace. Rust crates use
`publish = false`; TypeScript packages use `private: true`. Working names have
no verified registry ownership. There is no npm/crates.io install, hosted docs,
tag, or public release claim.

Private consumers use exact clean source receipts or produced archives and an
exact lock. Moving branches, dirty unrecorded sources, sibling workspace
resolution, and unpacked source aliases are not compatibility evidence.

## Current Compatibility Evidence

| Layer | Current checked claim |
| --- | --- |
| Longhorn version | coordinated private `0.1.0` |
| Rust | edition 2024; MSRV 1.90 with exact compatible lock |
| TypeScript/Svelte | Svelte 5; exact peer ranges are package-specific in the generated API surface |
| Tauri renderer API | `@longhorn/tauri` peer `^2.10.1`; greenfield proof resolves 2.10.1 |
| Tauri Rust | private migration artifacts prove the exact locked Tauri 2.11.5 graph on Rust 1.90 |
| Poodle | exact five-package private artifact receipt; no general registry range claim |
| protocol | exact v1 negotiation; checked generated TypeScript and cross-language fixtures |

`@longhorn/svelte` currently declares `>=5.38.6 <=5.56.8`. Other optional
Svelte adapters commonly declare `>=5.38.6 <6`; consult the
[generated API surface](../reference/api-surface.md), not a global inferred
range. The greenfield matrix proves one runtime at 5.38.6. Later consumer
proofs resolve 5.56.8 where recorded.

The current coordinated receipt is the Card 127
[private `0.1.0` candidate](../reference/private-0-1-candidate.md). It binds
the exact five-package Poodle set, all Longhorn packages, seven consumer
graphs, and two reproducible production runs. It does not claim compatibility
with every future Poodle `0.1.0` source state.

## Protocol Compatibility

Rust serialized types are authoritative. Generated TypeScript stays checked
in with its owning package and must regenerate at zero diff.

- additive optional fields may remain within a compatible protocol line
- removed fields, renamed serialized keys, changed enum/field meaning, or
  incompatible required fields are breaking
- unknown future enum variants return explicit incompatibility
- v1 negotiates exact protocol version; ranges wait for rolling-upgrade evidence
- request id is correlation, not replay permission
- capability and domain authority remain separate from protocol compatibility
- serialized-loopback parity proves codecs, not production network transport

A compatibility failure names package, local version, remote protocol,
supported version/range, failed feature, and recovery/upgrade route. It never
falls back to an unchecked local DTO or generic command bus.

## Platform Claims

Pure config, geometry, layout, protocol, history, operation, and notification
mechanisms are platform-neutral at their injected boundaries. A packaged host
claim requires target evidence.

| Host mechanism | macOS | Windows | Linux |
| --- | --- | --- | --- |
| Tauri window/transfer proofs | packaged macOS evidence | no packaged claim in current guide set | no packaged claim in current guide set |
| child view | packaged proof; live scale switch unavailable; focus/visibility may be unknown | unproved | unproved |
| isolated native window | packaged proof pass | unsupported | unsupported |
| backing surface | packaged proof with live scale transition unmet | unsupported | unsupported |

Cross-platform storage path resolution is deterministic for injected macOS,
Windows, and Linux facts. That does not convert a macOS-only native mechanism
into a portable host.

## Upgrade Procedure

1. Freeze current source, locks, artifact checksums, protocol fixtures, storage
   locator, and consumer behavior.
2. Produce the candidate packages; do not resolve against sibling workspaces.
3. Regenerate Rust-owned TypeScript and fail on drift.
4. Build the exact selected graph with the declared MSRV and peers.
5. Run direct, serialized, Tauri, mounted lifecycle, capability, and optional-
   edge conformance required by that consumer.
6. Inspect storage/profile/domain schema changes. Plan any migration backup-
   first and commit selectors last.
7. Run fresh start, existing-state restart, corrupt/future input, teardown,
   previous-build readback, and rollback.
8. Switch one bootstrap authority only after the candidate passes.
9. Retain previous source and durable state until explicit cleanup evidence.

An unconstrained fresh Cargo resolution is not MSRV proof; transitive crates
may raise their toolchain floor. Peer compatibility is proved by isolated
installation and one-runtime audit, not by a permissive manifest string alone.

## Versioning

Before 1.0, make clean coordinated changes. Do not add compatibility aliases,
deprecated stubs, re-export shims, or silent fallbacks. Coordinate material
consumer breaks and migrate call sites in the bounded lane.

First public publication must verify registry ownership, normalized package
artifacts, public names, final ranges, checksums, licensing, release notes, and
hosted surfaces. Card 127 produces a deterministic private candidate and runs
read-only gates only; it performs none of those publication actions.
