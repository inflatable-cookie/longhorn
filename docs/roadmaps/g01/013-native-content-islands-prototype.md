# g01.013 Native Content Islands Prototype

Status: complete
Owner: Tom
Updated: 2026-07-31
Governing refs: contracts 001, 003, 009, 010, 012, 013, and 017; research
memo 017

## Outcome

Prove a small desired/observed coordination seam across child webviews,
isolated native inspection windows, and embedded backing surfaces without
merging their host implementations.

## Goals

- [x] characterize current Nucleus, Soundcheck, and Jetstream mechanisms
- [x] promote shared and mechanism-specific authority into contract 017
- [x] prove one private pure coordination model against all three traces
- [x] prove three independent packaged macOS mechanism prototypes
- [x] record truthful Windows and Linux support per mechanism
- [x] choose promote, narrow, retain, or reject from measured evidence
- [x] compile public implementation only after the promotion decision

## Characterization Decision

Share identity, attach generation, typed client viewport, explicit scale,
presence, visibility, focus intent, input-routing mode, observation, planning,
and receipts.

Keep three separate mechanisms:

- Nucleus child webview
- Soundcheck isolated native helper window
- Jetstream backing GPU surface beneath a transparent webview

The viewport is semantic presentation and interaction geometry. It becomes
child bounds, isolated-window content size, or backing-surface clip according
to the selected mechanism.

## Execution Plan

### Batch 1. Characterization

- [x] audit Nucleus child-webview hosting read-only
- [x] audit Soundcheck isolated native-window hosting read-only
- [x] audit Jetstream backing-surface hosting read-only
- [x] check lifecycle, geometry, scale, visibility, focus, input, teardown,
  platform, and current Tauri API evidence
- [x] promote research memo 017 and contract 017

### Batch 2. Private pure coordination

- [x] [Card 082](batch-cards/082-private-native-content-coordination-prototype.md)
  builds the non-publishable identity, desired/observed state, planning,
  receipt, stale-generation, and three-shape trace prototype
- [x] prove 1x/2x typed conversion, unknown visibility, partial apply, and
  teardown without a Tauri, browser, plugin, GPU, Svelte, or Poodle edge

### Batch 3. Separate packaged mechanisms

- [x] [Card 083](batch-cards/083-child-webview-mechanism-packaged-prototype.md)
  proves child bounds, reuse, visibility, focus, close, scale, and remote
  capability confinement
- [x] [Card 084](batch-cards/084-isolated-native-window-packaged-prototype.md)
  proves fake-child content-size negotiation, helper loss, and bounded
  teardown without a plugin dependency
- [x] [Card 085](batch-cards/085-backing-surface-mechanism-packaged-prototype.md)
  proves full-host native backing, viewport clipping, forwarded-input gating,
  scale, destruction, and declared detach policy
- [x] prove each adapter graph excludes the other mechanisms

### Batch 4. Promotion decision

- [x] [Card 086](batch-cards/086-native-content-promotion-decision-and-closeout.md)
  compares all gates and chooses promote, narrow, retain, or reject
- [x] update package topology and public claims from evidence
- [x] compile a production runway only for promoted boundaries
- [x] close g01.013 without implying donor migration

## Acceptance Criteria

- [x] current donors and Tauri API surface are characterized read-only
- [x] shared and mechanism-specific ownership is explicit
- [x] one product-neutral trace vocabulary represents all three shapes
- [x] prototypes use current Tauri/native APIs in packaged macOS apps
- [x] scale, visibility or unknown occlusion, destruction, focus, and teardown
  are observable without fabricated success
- [x] unused mechanisms stay absent from isolated dependency graphs
- [x] Windows and Linux are classified as proved, unsupported, or unproved per
  mechanism without inflated claims
- [x] a production direction exists only because decision evidence shows real
  reuse; no production package is claimed yet
- [x] no donor product payload or raw native handle crosses shared protocols

## Promotion Decision

Card 086 selects `Promote`: one pure kernel, three separate mechanism layers,
checked TypeScript, and per-instance Svelte support. Poodle remains an external
public composition seam. Initial native hosts are macOS-only within recorded
scale limits. Cards 082-085 remain private evidence and cannot publish or
trigger donor migration. g01.018 is the separate production runway.

## Lane Runway

- generation goal met: native-content research has a single measured
  production disposition before consumer migration
- complete: Cards 082-085 evidence and Card 086 promotion decision
- complete: g01.018 Cards 087-092 production kernel, checked client,
  child-view, isolated-window, backing-surface, and Svelte adapters
- complete: Card 093 isolated artifacts, conformance, support, and adoption
  proof

## Next Task

Execute g01.014 Card 094. Its read-only fixtures and admission dossier precede
the compiled storage, window, workspace, Browser, and closeout cards.
