# g01.018 Native-content Production And Adoption Gate

Status: complete; Cards 087-093 complete
Owner: Tom
Updated: 2026-08-01
Governing refs: contracts 003, 009, 010, 012, 013, and 017; research memo
017; Card 086
Depends on: g01.013

## Outcome

Implement the promoted native-content coordination graph as production Rust,
TypeScript, and Svelte artifacts. Prove three separately selectable macOS host
mechanisms before any consumer migration.

## Package Direction

- `longhorn-native-content`
- `longhorn-tauri-native-content-child-view`
- `longhorn-native-content-isolated-window`
- `longhorn-native-content-backing-surface`
- `@inflatable-cookie/longhorn-native-content`
- `@inflatable-cookie/longhorn-native-content-svelte`

The names are working pre-1.0 names. No Poodle-specific package is planned.

## Execution Plan

### Batch 1. Production authority and client

- [x] [Card 087](batch-cards/087-native-content-identity-state-and-planning.md)
  implements the pure kernel from contract 017, not by promoting prototype
  source
- [x] [Card 088](batch-cards/088-generated-native-content-client-and-host-protocol.md)
  adds checked generation, a framework-neutral client, and narrow host
  protocol

### Batch 2. Independent mechanisms

- [x] [Card 089](batch-cards/089-tauri-child-view-production-adapter.md) adds
  the isolated Tauri child-view adapter and packaged macOS proof
- [x] [Card 090](batch-cards/090-isolated-window-production-adapter.md) adds
  generic isolated-window/process coordination and packaged macOS proof
- [x] [Card 091](batch-cards/091-backing-surface-production-adapter.md) adds
  generic backing-surface coordination and packaged macOS proof

### Batch 3. Renderer lifecycle

- [x] [Card 092](batch-cards/092-svelte-native-content-session-and-layout-seam.md)
  adds per-instance Svelte sessions, viewport measurement, explicit gates,
  and public-layout composition without a Poodle dependency

### Batch 4. Artifacts and adoption gate

- [x] [Card 093](batch-cards/093-native-content-artifact-proof-and-closeout.md)
  proves isolated artifacts, support claims, three-shape conformance, and
  migration prerequisites

## Goals

- [x] keep the pure graph free of Tauri, browser, plugin, GPU, Svelte, and
  Poodle dependencies
- [x] keep all three host mechanisms independently selectable
- [x] generate renderer protocol from Rust authority
- [x] preserve exact generations, revisions, partial receipts, and unknown
  observations
- [x] keep outer placement, product content, and semantic input external
- [x] publish no support claim beyond measured target evidence

## Acceptance Criteria

- [x] pure and selected-adapter graphs pass from clean produced artifacts
- [x] all three product-neutral traces match Rust and renderer behavior
- [x] each macOS mechanism passes packaged lifecycle and teardown proof
- [x] child/backing scale claims distinguish deterministic conversion from a
  live native display transition
- [x] public Svelte lifetime tears down every listener and observer
- [x] public Poodle composition uses only documented consumer seams
- [x] Nucleus, Soundcheck, and Jetstream prerequisites are explicit
- [x] no donor repository changes occur in this milestone

## Platform Baseline

- pure coordination: target-independent semantics; target artifacts required
  before portability claims
- child view: macOS first; Windows and Linux remain unproved
- isolated window: macOS only; Windows and Linux unsupported
- backing surface: macOS only; Windows and Linux unsupported

## Migration Gate

| Consumer | Artifact gate | Next admission | Remaining native-content gate |
| --- | --- | --- | --- |
| Nucleus | pass | g01.014 migration planning | browser construction/source, navigation/popup/download/permission/data-store policy, panel/overlay visibility policy, remote-child capabilities |
| Soundcheck | pass | after g01.014 and g01.015 | Signal/plugin/helper authorization and lifecycle mapping |
| Jetstream | pass | after g01.014 and g01.015 | native storage/WGPU, renderer, and semantic-input mapping |
| Loophole | not applicable | no current native-content migration | none claimed |

Artifact readiness does not authorize donor cutover.

## Next Task

Execute g01.014 Card 094. Freeze read-only behavior, Browser policy, rollback,
and donor-write admission as checked evidence. Cards 095-101 now carry the
cutover and closeout runway.
