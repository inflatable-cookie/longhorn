# Operation And Notification Artifact Proof And Closeout

Date: 2026-07-31
Card: 081
Roadmap: g01.012

## Result

Four clean consumers run from produced TypeScript artifacts and private Rust
inventory archives. Operation-only and notification-only graphs remain
independent. Soundcheck-shaped cancellation and Loophole-shaped queued render,
retained notification, bridge, Svelte, and Poodle composition preserve the
claimed behavior. g01.012 is complete.

## Artifact Evidence

TypeScript:

| Package | Version | SHA-256 |
| --- | --- | --- |
| `@inflatable-cookie/longhorn-core` | 0.1.0 | `b41c3585e48f8e538acaccc68209660e7be55962bdea0af443af924011fcc9f0` |
| `@inflatable-cookie/longhorn-tauri` | 0.1.0 | `4df629c1bc5ebd889bdb29e100956f0a2327fc4fc7cee38843b45a086f7fa3c5` |
| `@inflatable-cookie/longhorn-bridge` | 0.1.0 | `b492c930fe1c2e03d65fce49ec5d928f6b6370e535970451d4b178fa756ba9a8` |
| `@inflatable-cookie/longhorn-operation` | 0.1.0 | `775f8747ab8a671311f5ecea1b22d49196bf39dfa708b9f62291888bf4d79657` |
| `@inflatable-cookie/longhorn-notifications` | 0.1.0 | `b4d59137ab7b1c7196aa73ee36d79481003ff73580bc9a181d4a690bbfd5e95d` |

Exact Poodle artifact set:
`39f08c04fa2579ae709db412c28221c04f22b89f09e633cef93764e5d49f8c74`.

Private Rust inventories cover core, bridge, operation, notifications, and
both narrow Tauri adapters. Every crate passes `cargo package --list
--allow-dirty`; exact sources unpack into a temporary workspace and run
offline on Rust 1.85. Registry-normalized `.crate` artifacts remain a release
gate.

## Graph Evidence

| Shape | Rust | TypeScript | Transports |
| --- | --- | --- | --- |
| Minimal operation | core, operation | core, operation | direct, serialized |
| Soundcheck | core, operation, Tauri operation | core, operation | direct, Tauri |
| Loophole | core, bridge, operation, notifications, both Tauri adapters | core, bridge, Tauri, operation, notifications | direct, Tauri, bridge |
| Notification-only | core, notifications, Tauri notifications | core, notifications | direct, serialized, Tauri |

The minimal operation install has no notifications, bridge, Tauri package,
Svelte, Poodle, config, settings, commands, or history. Notification-only has
no operation or command package. Locks contain no workspace, link, or sibling
source resolution.

## Semantic Evidence

| Area | Result |
| --- | --- |
| parity | Rust-produced fixtures and renderer clients emit equal public traces |
| Soundcheck | running → cancelling → cancelled; repeated cancellation does not advance truth; late terminal progress rejects without change |
| Loophole | queued → running → progressed → succeeded across direct, Tauri, and bridge ports |
| notifications | operation and service-reliability records share one retained ledger without sharing operation state |
| windows | two mounted sessions remain distinct over one authority |
| remount | retained operation and notification truth reloads; old toasts do not replay |
| toast | expiry removes transient presentation only |
| actions | an action admitted when rendered is rejected after current consumer admission changes |
| teardown | all mounted notification listeners unregister exactly once |
| UI | packed Svelte consumers use one runtime and the exact public Poodle artifact set |

## Boundary Audits

- generated operation and notification bindings have zero drift
- operation never depends on notifications
- notification-only installs never acquire operation or command packages
- no generic executor, queue, retry, report, artifact, warning, or log payload
  enters the shared protocols
- executor, queue, retry, retention, wording, semantic actions, and migration
  remain named consumer authorities
- capabilities match only selected reads, mutation, cancellation, and event
  listening
- no donor repository was written

## Validation

- `effigy proof:operation-notification-artifacts`
- four clean TypeScript installs and executable semantic traces
- four offline Rust consumer runs on Rust 1.85
- one packed mounted Svelte/Poodle multi-window and remount test
- full `effigy qa`

## Closeout

Cards 075-081 and g01.012 are complete. The next bounded runway task is
g01.013 native-content-island characterization. Consumer migration remains
blocked behind its named roadmap gates; Card 081 did not alter donor apps.
