# 034 Generated Transfer Protocol And Tauri Host

Status: complete
Owner: Tom
Roadmap: g01.006 batch 5
Governing refs: contracts 001, 002, 009-014; research memo 010
Depends on: Cards 031-033
Auto-start next card: no

## Objective

Expose checked Surface and transfer protocols through framework-neutral
TypeScript packages and one testable Tauri handler assembly.

## Scope

- `longhorn-bindings` Surface and transfer generation
- checked `@longhorn/surfaces`, `@longhorn/transfer`, and
  `@longhorn/surface-transfer` packages
- protocol compatibility guards
- framework-neutral session, lease, commit, cancel, and snapshot clients
- `longhorn-tauri-transfer`
- one handler assembly for real and mock runtimes
- managed-window readback and checked screen-space projection
- client epoch, listener-before-snapshot, and teardown behavior
- narrow capability examples

## Public Behavior

Rust serde remains authoritative. Raw Tauri invoke/listen stays inside the
host client. Renderer leases are complete replacements bound to one client
epoch and managed window.

No Svelte store or Poodle component ships in this card. The clients expose
protocol behavior needed by the packaged proof and later g01.007 adapters.

## Out Of Scope

- reusable Svelte stores or actions
- Poodle binding behavior
- keyboard drag UX
- consumer panel rendering
- donor migration
- registry publication

## Steps

1. Generate Surface snapshots, requests, receipts, and errors.
2. Generate transfer session, lease, target, completion, and abort types.
3. Add explicit protocol compatibility guards.
4. Add framework-neutral typed clients.
5. Assemble Tauri commands and events once for real and mock runtimes.
6. Bind handlers to current managed-window identity and readback.
7. Project and validate client drop-zone geometry explicitly.
8. Add epoch, teardown, stale, and capability behavior.
9. Add Rust/TypeScript golden and mock-runtime fixtures.
10. Check package contents, import safety, and zero-diff generation.

## Acceptance Criteria

- generated files exactly match Rust authority
- future protocol and unknown variants fail explicitly
- raw Tauri calls remain inside the bridge client
- listener-before-snapshot cannot miss a current epoch
- late registration teardown does not leak
- renderer cannot lease for another managed window
- invalid geometry publishes no replacement lease
- no Svelte, Poodle, product, or donor dependency ships
- package dry runs contain only intended files
- Tauri capability examples grant only named commands

## Evidence Required

- zero-diff generation
- Rust/TypeScript golden fixtures
- import-safety and package-content tests
- mock-runtime handler matrix
- epoch and teardown fixtures
- capability audit
- Rust 1.85 and full Effigy QA

## Stop Conditions

- handwritten duplicate DTOs are required
- Tauri handlers become a product command bus
- client geometry bypasses checked projection
- reusable UI policy is required
- package names require registry authority before local implementation

## Next Task

Card 035 is ready. Prove the shared protocol and handler in one packaged
multi-window app, then close g01.006.

## Outcome

`longhorn-bindings` now generates checked Surface, transfer, and optional
Surface-transfer protocols and Rust-authored golden fixtures.
`@longhorn/surfaces`, `@longhorn/transfer`, and
`@longhorn/surface-transfer` provide compatibility guards and
framework-neutral clients. `@longhorn/tauri` is the sole raw invoke/listen
adapter.

`longhorn-tauri-transfer` binds commands to current managed-window readback,
projects client geometry through checked scale conversion, and shares one
coordinator and client epoch across base and optional Surface handlers. The
renderer connection installs its epoch listener before snapshot invoke,
accepts only the newest epoch, and tears down safely when listener
registration resolves late.

Base and Surface capability examples grant only the named commands and event
listen/unlisten. Rust and TypeScript protocol, handler, connection,
import-safety, package-content, capability, Rust 1.85, and full Effigy QA
evidence passes. The closeout report is
[`Generated Transfer Protocol And Tauri Host`](../../../logs/2026-07/29-generated-transfer-protocol-and-tauri-host.md).
