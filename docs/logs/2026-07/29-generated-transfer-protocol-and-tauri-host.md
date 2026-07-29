# Generated Transfer Protocol And Tauri Host

Date: 2026-07-29
State: complete implementation batch

## Outcome

- completed Card 034
- generated checked Surface, transfer, and optional Surface-transfer protocols
  from Rust serde authority
- added framework-neutral snapshot, session, lease, commit, cancel, and
  Surface-transfer clients
- rejected incompatible protocol versions and unknown variants explicitly
- added `@longhorn/tauri` as the sole raw invoke/listen adapter
- added `longhorn-tauri-transfer` with real and mock command assembly
- bound renderer operations to the caller's current managed-window identity
- projected client geometry through checked scale and window readback
- shared one coordinator and client-epoch authority across base and optional
  Surface commands
- added listener-before-snapshot connection and teardown-safe late listener
  cleanup
- added exact base and Surface capability examples
- made Card 035 ready

## Package Boundary

| Package | Authority |
| --- | --- |
| `@longhorn/surfaces` | generated Surface snapshots, requests, receipts, and compatibility |
| `@longhorn/transfer` | generated transfer protocol and framework-neutral connected client |
| `@longhorn/surface-transfer` | optional whole-Surface transfer client |
| `@longhorn/tauri` | raw Tauri invoke/listen transport only |
| `longhorn-tauri-transfer` | caller binding, geometry projection, command/event assembly |

No Svelte, Poodle, donor, or product package enters this slice. The base Rust
adapter does not link `longhorn-surface-transfer`; the feature adds Surface
commands over the existing shared coordinator.

## Epoch And Teardown

The Tauri snapshot command issues and emits one `TransferClientSnapshot`. The
renderer installs its event listener before invoking the snapshot command and
retains only the newest epoch. Disposal is idempotent. If listener
registration completes after disposal, the returned unlisten callback runs
without applying a snapshot.

## Capability Audit

The base example grants the five base transfer commands plus core event listen
and unlisten. The Surface example adds exactly the two optional Surface
commands. Rust tests audit both files.

## Validation

- `effigy test:tauri-transfer`: 7 passed
- `effigy test:transfer-ts`: 8 passed
- `effigy test:tauri-ts`: 3 passed
- `effigy check:transfer-bindings`: zero diff
- `effigy check:tauri-package`: intended three-file package
- focused Rust clippy: passed
- Rust 1.85 focused check: passed
- full `effigy qa`: passed

`effigy doctor` retains one pre-existing high-size Tauri lifecycle error and
expected generated-source warnings. Splitting the new transfer handler into
panel, Surface, and shared-type modules removed its structural warning.

## Posture

`strict-ready`

## Next

Start Card 035: prove both composition modes through the packaged
multi-window runtime, audit platform-limited evidence, and close g01.006.
