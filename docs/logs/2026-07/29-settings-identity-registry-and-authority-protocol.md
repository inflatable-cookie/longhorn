# Settings Identity, Registry, And Authority Protocol

Date: 2026-07-29
Card: 042
State: complete

## Outcome

- added `longhorn-settings` as a pure Rust workspace crate
- added bounded settings module, section, page, renderer, anchor, scope,
  apply-unit, capability, activation-target, entry, request, policy-source,
  and authority-token ids over `longhorn-core`
- implemented mutable-before-seal registration with explicit limits
- validated owners, sections, renderers, scopes, apply units, capabilities,
  text, references, and globally stable anchors before admission
- admitted pages through composed capabilities and removed empty sections
- ordered modules, sections, pages, anchors, and references canonically
- separated host-monotonic registry generation from content-derived SHA-256
  digest
- added configured, effective, default, policy, editability, diagnostic,
  recovery, durability, and activation projections
- added strict load, apply, reset, conflict, rejection, snapshot, and receipt
  protocol types
- kept consumer values opaque through bounded versioned JSON envelopes

## Registry Boundary

Registration order carries no authority. Duplicate category ids fail at
registration. Seal validates the complete declaration set before capability
admission, so an unavailable page cannot hide a missing owner or dependency.
Equal explicit order uses stable id as the tie-break.

The host supplies a monotonic generation for each composition. The digest
excludes generation and hashes canonical limits, admitted content, composed
capabilities, and declarations. Equivalent content registered in another
order retains one digest. Capability admission changes the digest.

## Authority Boundary

Authority identity contains registry generation, scope revision, and one
host-issued opaque token. Apply and reset bind the page, one failure-atomic
apply unit, one scope, checked authority, and consumer intent. The pure crate
does not compare tokens, interpret values, mutate storage, or claim
cross-domain atomicity.

Configured user state, effective state, compiled default, policy provenance,
and editability are independent fields. Successful mutation receipts report
durability and runtime activation separately. Restart is not encoded as a
mutation timing.

## Donor Fixtures

| Shape | Proof |
| --- | --- |
| Split-shell | one staged page, one scope, one apply unit, no optional system |
| Soundcheck | immediate product state beside staged library and capability-gated recovery |
| Loophole | immediate appearance and hardware beside staged keybindings; specialist authority stays consumer-owned |

No donor field schema, renderer implementation, storage path, Tauri command,
Svelte state, or Poodle component entered the crate.

## Evidence

- 22 settings contract fixtures pass
- bounded ids and strict serde round trips pass
- duplicate, missing-reference, ownership, limit, and capability rejection
  paths pass
- canonical order and digest remain stable across registration order and
  registry generation
- future protocol versions and unknown command fields fail
- opaque value codec, configured/effective/policy, stale authority,
  durability, and activation round trips pass
- `cargo tree -p longhorn-settings --edges normal` contains only
  `longhorn-core`, Serde/JSON, SHA-256, and their external support graph
- no config, layout, Surface, command, history, Tauri, Svelte, Poodle, or
  consumer dependency appears

## Validation

- focused Rust format, Clippy, tests, and doc tests passed
- focused settings contract suite passed
- full Effigy QA passed
- `git diff --check` passed
- Effigy Doctor retains the known window-lifecycle god-file baseline; Card 042
  adds no finding

## Current State

Card 042 is complete. Card 043 is ready and not started.

## Next

Start Card 043. Prove token comparison and one-domain mutation occur under one
configuration coordinator authority before implementing the adapter.
