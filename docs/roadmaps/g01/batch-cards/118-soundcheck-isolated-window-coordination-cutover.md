# 118 Soundcheck Isolated-window Coordination Cutover

Status: complete
Owner: Tom
Roadmap: g01.016 batch 2
Governing refs: contracts 003, 009-010, 012-013, and 017; Card 117
Depends on: Card 117
Auto-start next card: no

## Objective

Adopt Longhorn's isolated-window desired/observed coordination around
Soundcheck's plugin-inspection helper without taking plugin, helper, or native
window policy away from Soundcheck and Signal.

## Repository Scope

- Longhorn: isolated-window adapter, conformance fixture, evidence, and docs.
- Soundcheck: helper coordination seam, renderer client, tests, and docs.
- Signal and soundcheck-library: read-only authority checks.

## Scope

- stable island identity, attach generation, desired content size, visibility,
  focus, and teardown
- helper spawn correlation, readiness, startup progress, cancellation, failure,
  parent keepalive, and terminal observation
- plugin-requested and user-requested size proposals and receipts
- exact disposable-process close and unsafe-unload policy
- native titlebar/header and screenshot behavior
- macOS-only support statement and unsupported platform behavior

## Steps

1. Freeze helper startup, readiness, size, close, crash, and cancellation traces.
2. Bind one isolated-window coordination record to each helper generation.
3. Adapt helper launch and observed state through injected consumer ports.
4. Route size proposals through checked generation and product acceptance.
5. Preserve immediate process exit and parent-keepalive semantics.
6. Keep plugin view, header, audio/MIDI, screenshots, and format behavior local.
7. Add checked renderer observation without exposing plugin payloads.
8. Remove only superseded generic coordination code.

## Acceptance Criteria

- Longhorn owns identity, generation, desired/observed state, planning, and receipts
- Soundcheck owns authorization, helper lifecycle policy, native header, and disposable exit
- Signal owns plugin ABI, editor, audio/MIDI, processing, and size constraints
- stale helper generations cannot mutate current state
- cancellation and crash are visible terminal outcomes
- native scale support is claimed only where proved
- Windows/Linux remain unsupported, not silently degraded
- no raw native pointer or plugin payload crosses Longhorn's public boundary

## Stop Conditions

- generic coordination would need to load or unload a plugin
- disposable-process correctness conflicts with shared teardown
- a raw pointer or ABI type must enter Longhorn
- unsupported platforms would appear successful

## Next Task

Execute Card 119. Prove and close the complete Soundcheck migration.
