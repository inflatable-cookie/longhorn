# 090 Isolated-window Production Adapter

Status: complete
Owner: Tom
Roadmap: g01.018 batch 2
Governing refs: contracts 003, 009, 012, and 017; Card 086
Depends on: Card 087
Auto-start next card: no

## Objective

Implement generic isolated-window and process-boundary coordination over
consumer-owned native content. Preserve Soundcheck's safe disposable-helper
shape without importing Signal or plugin authority.

## Scope

- `longhorn-native-content-isolated-window`
- injected owner/content lifecycle ports
- content-area sizing and consumer acceptance receipts
- show, hide, focus loss, close, helper loss, and stale-message handling
- bounded request timeout and owner termination teardown policy
- macOS packaged fake-child proof
- explicit Windows/Linux unsupported ledger

## Out Of Scope

- plugin discovery, ABI, authorization, audio, MIDI, state, or screenshots
- unsafe third-party unload
- outer-window placement
- child-view, backing-surface, Svelte, or Poodle
- Soundcheck migration

## Acceptance Criteria

- content-size proposals cannot directly rewrite durable desired state
- outer position has no adapter operation
- helper messages carry generation and bounded correlation
- timeout, helper loss, stale response, and owner termination are explicit
- no raw pointer crosses process or renderer protocol
- packaged macOS proof retains the Card 084 behavior matrix
- unused host mechanisms remain absent from the graph

## Evidence Required

- fake-child resize and lifecycle transcript
- timeout, helper-loss, stale, and termination fixtures
- native-boundary and dependency inventory
- target support ledger
- focused Rust, packaged, docs, and Effigy checks

## Stop Conditions

- a plugin ABI or Signal type must enter the shared package
- safe teardown requires generic invocation of unsafe unload
- outer placement becomes adapter authority
- current packaged behavior regresses from Card 084 evidence

## Next Task

Execute Card 091. Implement the backing-surface coordination layer over
consumer-supplied storage, rendering, and input ports.
