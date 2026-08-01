# 082 Private Native-content Coordination Prototype

Status: complete
Owner: Tom
Roadmap: g01.013 batch 2
Governing refs: contracts 001, 003, 009, and 017; research memo 017
Depends on: Card 081 and the completed g01.013 characterization batch
Auto-start next card: no
Completed: 2026-07-31

## Objective

Build a non-publishable pure prototype for native-content identity,
desired/observed state, lifecycle planning, and exact receipts. Prove one
vocabulary can describe child-view, isolated-window, and backing-surface
traces without pretending they share one host implementation.

## Scope

- standalone `prototypes/native-content` Rust workspace
- opaque island and bounded consumer-kind identity
- host-window binding, coordination revision, and attach generation
- closed mechanism and capability descriptions
- host-local `ClientCssPx` viewport and explicit `ScaleFactor`
- desired presence, visibility, focus intent, and input-routing mode
- observed attachment, readiness, visibility, focus, and geometry
- ordered attach, update, detach, and failure plans
- applied, failed, and dependency-skipped operation receipts
- content-size proposals and consumer acceptance decisions
- Nucleus-, Soundcheck-, and Jetstream-shaped product-neutral traces

## Prototype Behavior

The desired viewport means where native content is presented and interactive.
The selected mechanism maps that meaning to child bounds, isolated-window
content size, or backing-surface render/input clipping.

All renderer measurements, adapter completions, platform observations, and
teardown evidence name the current attach generation. Stale and future
generations reject without state change. Successful operation calls do not
fabricate observed native truth.

The prototype may compare representations. It cannot export a stable crate,
join the root workspace, generate public TypeScript, or become a donor
dependency.

## Out Of Scope

- Tauri, native handles, platform APIs, or packaged desktop execution
- browser navigation, data stores, downloads, popups, or capabilities
- plugin ABI, process supervision, audio, MIDI, presets, or unsafe unload
- GPU devices, render loops, scenes, cameras, picking, or gizmos
- generic pointer, keyboard, plugin, MIDI, or render payloads
- Svelte or Poodle adapters
- production package names or compatibility promises
- donor repository writes

## Steps

1. Create the standalone private workspace with no root-workspace membership.
2. Add bounded island, kind, revision, generation, and host-binding types.
3. Define closed mechanism capabilities and input-routing modes.
4. Define desired and observed state with typed viewport and scale evidence.
5. Implement validation, stale-generation rejection, and lifecycle planning.
6. Implement ordered operation outcomes and fresh-observation convergence.
7. Add bounded content-size proposal and acceptance records.
8. Freeze three product-neutral donor-shaped semantic traces.
9. Exercise scale, zero viewport, unknown visibility, failure, and teardown.
10. Measure dependency and public-symbol isolation.

## Acceptance Criteria

- one exposed prototype API represents all three traces without donor payloads
- native frame and semantic viewport remain distinct
- child-view plans drive bounds; backing-surface plans drive clipping
- isolated-window proposals cannot mutate outer placement authority
- stale or future generations leave exact state unchanged
- every partial plan exposes applied, failed, and dependency-skipped work
- unknown visibility is never inferred as hidden or visible
- focus request success does not fabricate observed focus
- detach is idempotent and unresolved teardown remains explicit
- 1x and 2x conversions use checked shared geometry and rounding
- the graph imports no Tauri, browser, plugin, GPU, Svelte, or Poodle code
- no public workspace package, generated client, or compatibility promise exists

## Evidence Required

- three-shape desired/observed trace fixtures
- lifecycle and legal-transition matrix
- stale/future generation and revision failure matrix
- viewport/scale/rounding fixtures at 1x and 2x
- visibility, focus, content-size, and teardown fixtures
- partial-apply and fresh-observation receipts
- public-symbol and dependency audit
- focused Rust, clippy, formatting, docs, and Effigy checks

## Packaged-proof Gate

This card does not satisfy native packaged proof. Cards 083-085 must apply this
model through three independent packaged mechanisms. Card 086 decides whether
any production package is justified.

## Stop Conditions

- one trace requires browser, plugin, GPU, or product input payloads in core
- the viewport cannot remain semantically distinct from a native child frame
- one mechanism requires outer-window placement inside this authority
- stale native completions cannot be rejected without ambient process state
- the pure graph requires Tauri or a mechanism adapter
- multiple materially different coordination contracts remain equally valid

## Next Task

Execute ready Card 083. Apply the frozen trace to an independently packaged
child-webview mechanism.
