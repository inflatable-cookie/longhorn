# Authoritative Layout Panel Transfer

Date: 2026-07-29
State: complete implementation batch

## Outcome

- completed Card 032
- extended `longhorn-transfer` with the registered layout adapter
- admitted sessions only after fresh movable-panel resolution
- projected direct-window and Surface-container hosts through opaque bindings
- consumed every first terminal attempt before adapter validation or mutation
- required source and target to name one registered layout domain and revision
- reloaded current host bindings, layout authority, source placement, target
  region, and insertion evidence
- committed only through the existing expected-revision `MovePanel`
- returned the existing authoritative layout and configuration receipt
- rejected cross-document and copy attempts before publication
- preserved exact current bytes on every post-admission abort
- made Card 033 ready

## Authority Flow

```text
fresh registered layout + fresh host binding
  -> movable panel admission
  -> bounded id-only session
  -> current lease and managed-window resolution
  -> consumed terminal attempt
  -> fresh host-binding and same-domain checks
  -> current layout reload
  -> expected-revision MovePanel
  -> atomic configuration publication
```

The session and lease remain evidence only. They never become layout mutation
authority.

## Host Bindings

`PanelHostBindings` is a complete fresh snapshot keyed by
`TransferHostBindingId`. Each entry maps one managed window to one registered
layout domain and container.

The two supported projections are:

- direct window to layout container
- Surface-hosted window to layout container

Both projections use the same adapter. `longhorn-transfer` imports no Surface
package or `SurfaceId`.

## Commit Rules

- only `Move` is supported
- `Copy` consumes the terminal attempt and fails before publication
- source and target document ids must equal the registered domain id
- recorded source and target revisions must agree
- the current registered document must still have that exact revision
- source panel, placement, movability, and host binding are re-resolved
- target window, host binding, container, region, capability, and insertion
  are re-resolved
- absent insertion means append against the current target-region length
- the existing layout engine retains placement, instance-policy, and candidate
  validation authority
- any race after the read-only reload is rejected by expected revision under
  coordinated publication

Valid same-document moves advance exactly one layout revision.

## Failure Invariance

The evidence covers:

- unknown and non-movable admission sources
- duplicate and stale host bindings
- disappeared target windows
- cross-document targets
- explicit copy requests
- stale source and target revisions
- disappeared source panels
- missing and ineligible target regions
- invalid insertion positions
- instance-policy-invalid current documents preserved for recovery
- replay after both success and failed terminal attempts

Failures before any prior durable layout mutation create no file. Failures
after an intervening authoritative mutation preserve its exact bytes.

## Package Report

| Package | Ordinary dependencies |
| --- | --- |
| `longhorn-transfer` | core, config, layout, layout-config, serde |

The direct Nucleus compile proof now includes `longhorn-transfer` without any
Surface dependency. The Loophole-shaped fixture uses the opaque
Surface-container binding and retains the exact target container.

## Validation

- `cargo test -p longhorn-transfer`: 24 passed
- `cargo clippy -p longhorn-transfer --all-targets -- -D warnings`: passed
- `cargo +1.85.0 check -p longhorn-transfer --all-targets`: passed
- direct and Surface-container conformance matrix: passed
- Surface-package and `SurfaceId` import scan: empty
- `effigy qa`: passed
- `effigy qa:northstar`: passed
- `cargo +1.85.0 check --workspace --all-targets`: passed
- `effigy graph index --json`: passed with no diagnostics or failed paths
- Nucleus ordinary dependency scan for Surface packages: empty
- `git diff --check`: passed

`effigy doctor` retains repository debt outside this card: one generated-source
warning and one high-size Tauri lifecycle error. It reports no Card 032 file;
the largest new source or test file is 239 lines.

## Remaining Limits

- no cross-document transaction
- no panel copy
- no whole-Surface transfer
- no target provisioner
- no Tauri handler or renderer client
- no donor migration

These remain assigned to Cards 033-035 and later adoption roadmaps.

## Posture

`strict-ready`

## Next

Start Card 033: add optional whole-Surface transfer and explicit receipted
window provisioning without coupling the panel path to Surface authority.
