# Soundcheck Isolated-window Coordination Cutover

Date: 2026-08-02
Roadmap: g01.016
Card: 118
State: complete; Card 119 ready

## Result

Soundcheck's disposable plugin-inspection helper now runs behind Longhorn's
isolated-window coordinator. Each launch receives one stable island and host
identity plus exact attach generation. The adapter listener exists before the
owner process launches. Startup readiness carries fresh content size,
visibility, focus, owner pid, and native-attachment evidence.

The plugin's first real editor size remains authoritative. A differing
`480x360` reservation produces an honest partial bootstrap receipt; Soundcheck
then establishes desired state from fresh helper observation. Plugin and user
resize requests enter a bounded generation-checked queue. Signal accepts or
constrains each request first. Longhorn records the original proposal, exact or
constrained decision, later desired update, apply receipt, and fresh
observation. A queue gap fails terminally instead of dropping resize evidence.

Normal plugin or user close publishes a close request and still uses immediate
process exit. Longhorn's detach policy reports completed exit or bounded owner
termination; it never asks Soundcheck to unload plugin code. Nonzero helper
exit becomes terminal failed observation. Startup cancellation, 30-second slow
warning, two-minute timeout, parent-stdin keepalive, screenshot monitoring, and
Browse usage remain unchanged.

The renderer gets an observation-only Longhorn client. Existing Poodle/library
contracts keep their `Promise<void>` launch shape; explicit island-returning
APIs support checked connect and snapshot without exposing desired mutation or
size-decision methods.

## Authority

- Longhorn: island identity, generation, desired/observed state, plans,
  receipts, helper event admission, and renderer session checks
- Soundcheck: authorization, helper launch, startup policy, parent keepalive,
  native titlebar, screenshots, Browse usage, and disposable exit
- Signal: ABI, editor, state, audio/MIDI, resize acceptance, and resize hints

No product id, plugin payload, native pointer, ABI object, or unload operation
enters Longhorn's public protocol or Soundcheck coordination state.

## Exact Evidence

- Longhorn source: `8f04703fbd2fc6c3806eca6ed8f591dd94c670ce`
- prior Soundcheck: `2faae9a94182283b2539c4ad16a9a9fad52e50c3`
- Soundcheck cutover: `ef1b939dedeae0474a5197a183970d6d5efdef57`
- retained Signal: `4c5b60681071095aedbf9013720e76c1c89e91ed`
- Soundcheck Cargo lock SHA-256:
  `cfab9d96b6261234392fd8fc3fb17b3b37c10165d0e906ec0e6ccc6cc0b11295`
- Soundcheck Bun lock SHA-256:
  `b64817c5db0f296859b350eb228f950669c119eddaf5adc4fcf78a9c225c6d47`
- fixture:
  `fixtures/migration/soundcheck-card118/isolated-window-coordination-cutover-v1.json`
- verifier: removed 2026-08-10 — Longhorn no longer keeps consumer-aware proofs; the recorded fixture is the retained evidence

## Validation

Soundcheck passes 111 app-library tests with two authenticated-Codex tests
ignored, 30 renderer tests, production Svelte checking, and production
bundling. Longhorn's native-content core, TypeScript client, and production
isolated-window adapter gates pass through the Card 118 selector.

The wider Soundcheck Effigy workspace run exposes two unrelated existing
`soundcheck-sync` expectation failures: REAPER host-key preference and Studio
Pro recovery capability. Card 119 owns the full-product conformance gate and
will resolve or rebaseline them from product contracts.

No live library was mutated. Plugin start, resize, focus, close, crash, parent
death, titlebar, screenshot, cancellation, and native scale traces remain Card
119 work. macOS is the only admitted plugin-inspection platform. Windows and
Linux remain explicit unsupported outcomes. Scale stays at Signal's proved
native 1:1 editor coordinate contract.

## Next

Execute Card 119. Run the complete Soundcheck artifact, storage, window,
settings, scan, helper, rollback, and retained-authority closeout before any
Split-shell write.
