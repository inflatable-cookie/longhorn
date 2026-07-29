# Whole-Surface Transfer And Window Provisioning

Date: 2026-07-29
State: complete implementation batch

## Outcome

- completed Card 033
- added optional `longhorn-surface-transfer`
- admitted sessions from fresh registered Surface and primary-host authority
- added explicit existing-window and empty-display target policy
- committed only through expected-revision `MoveSurface`
- retained the exact Surface-to-layout-container binding
- kept layout contents immutable and outside transfer payloads
- required empty-display targets to name predeclared participating windows
- admitted policy before any native creation
- modeled hidden creation, placement, readiness, commit, and cleanup receipts
- invoked cleanup after failed Surface publication
- returned typed host reconciliation for failed cleanup or post-publication
  host commit
- made Card 034 ready

## Authority Flow

```text
fresh Surface domain + fresh source binding
  -> bounded id-only session
  -> consumed terminal target resolution
  -> current source, revision, target, binding, and policy checks
  -> optional hidden creation + placement + readiness
  -> expected-revision MoveSurface
  -> atomic Surface publication
  -> optional host commit
```

The logical target exists in Surface topology before provisioning. Transfer
does not add windows, clone layout, repair preferences, or infer product
metadata.

## Empty-display Boundary

The shared coordinator now distinguishes:

- a point outside every fresh managed window: eligible for Surface policy
- a point inside a managed window without current lease or zone authority:
  ordinary consumed failure
- overlapping managed windows: ambiguity

Provisioning policy supplies current display bounds, target `WindowId`, exact
`WindowPlacement`, and optional insertion. Disabled is the default.

## Failure Evidence

| Failure | Durable result | Host result |
| --- | --- | --- |
| target or policy rejection | unchanged | no call |
| provisioning | unchanged | stage failure |
| Surface publication | current authority retained | cleanup receipt |
| cleanup | current authority retained | reconciliation required |
| host commit | Surface move committed | reconciliation required with publication |

The concurrent-revision fixture mutates the registered Surface after hidden
provisioning but before move publication. Expected revision rejects the move,
cleanup runs, and the exact intervening authority remains.

## Donor Delta

Loophole's whole-Surface screen-point movement and optional empty-display host
remain. Shared behavior removes first-match overlap, silent topology repair,
and product-shaped window creation. Consumer policy supplies targets and
placement. The adapter supplies no URL, title, role, chrome, or capability
default.

## Validation

- `cargo test -p longhorn-transfer -p longhorn-surface-transfer -j 1`: 34
  passed
- `cargo clippy -p longhorn-transfer -p longhorn-surface-transfer
  --all-targets -j 1 -- -D warnings`: passed
- `cargo +1.85.0 check -p longhorn-transfer
  -p longhorn-surface-transfer --all-targets -j 1`: passed
- `effigy qa`: passed
- `effigy qa:docs`: passed
- `effigy qa:northstar`: passed
- `effigy graph index --json`: passed with no diagnostics or failed paths
- `git diff --check`: passed
- layout-binding preservation, policy-off, target-loss, provision failure,
  cleanup success/failure, and post-publication reconciliation fixtures:
  passed

`effigy doctor` retains pre-existing repository debt: one high-size Tauri
lifecycle file and one generated-source warning. Card 033 adds neither.

## Posture

`strict-ready`

## Next

Start Card 034: generate checked Surface and transfer clients and assemble the
narrow Tauri host against the now-complete Rust commit contracts.
