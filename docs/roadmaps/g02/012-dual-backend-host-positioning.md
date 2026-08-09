# g02.012 Dual-backend Host Positioning

Status: complete
Completed: 2026-08-09
Owner: Tom
Updated: 2026-08-09
Governing refs: contract 020; contracts 018 and 012; research memo 021
Depends on: none

## Outcome

Longhorn is a Rust desktop application framework with two first-class,
permanent host backends. An application chooses Tauri or GPUI, composes the
same host-agnostic core, and pays only for the host adapter it selects.

## Generation Runway

Twelfth g02 milestone. Opened by measurement: 12.5% of the Rust is
Tauri-bound and the domain crates are already neutral, so the work is
finishing a separation that mostly exists rather than creating one.

## Execution Plan

### Batch 1. Make the existing separation honest

- [x] [Card 161](batch-cards/161-host-tier-positioning-and-leak-fixes.md)
  restates positioning, tiers the contracts, and closes the two leaks

### Batch 2. Delegated capability

- [x] [Card 162](batch-cards/162-native-update-execution.md) builds the
  update installer — for every host, as it turned out — under one
  conformance suite

### Batch 3. Second backend

- [x] [Card 163](batch-cards/163-gpui-host-adapter.md) implements the
  minimal GPUI host adapter that validates or refutes contract 020

## Dependency Shape

```text
memo 021 dual-backend positioning
 └─ 012 dual-backend host positioning
     ├─ 161 positioning and leak fixes   (independent)
     ├─ 162 native update execution      (independent)
     └─ 163 GPUI host adapter            (validates 161's contract)
```

## Goals

- [x] no pure crate depends on a host adapter
- [x] no host-shaped concept sits unqualified in `longhorn-core`
- [x] contract tiers are stated, and webview-edge contracts are optional
- [x] update installs on a host with no plugin, under the same contract
- [x] one host-contract claim is proved against both backends

## Acceptance Criteria

- [x] `longhorn-windowing-config` compiles with no `longhorn-tauri-*`
  dependency
- [x] every host-contract requirement is stated without naming a backend
- [x] ~~the native installer and the Tauri plugin path share one conformance
  suite~~ — superseded 2026-08-09. The plugin cannot implement the suite at
  all, so execution became host-independent and there is one path.
- [x] the GPUI adapter's unproven claims are recorded as unproven

## Explicit Non-goals

- migrating Loophole or Nucleus. Both convert later; neither is near-term.
- deprecating Tauri. It stays first-class permanently: the fast path for
  prototyping and for applications that never justify conversion.
- a lowest-common-denominator host abstraction. Differences are stated, not
  erased.
- porting `packages/*`. A GPUI application composes a projection tier, not a
  parallel client tier.

## Next Task

The milestone is closed. One follow-up card came out of it:
[Card 168](batch-cards/168-installation-provenance.md), which fixes a live
defect — a Homebrew-managed application currently self-updates and desyncs
`brew` silently.

Earlier follow-ups, all now done:

- [x] Split `HostCapability::MoveResize` into `Move` and `Resize`, so a host
  with one axis and not the other can declare it. GPUI now resizes for real.
- [x] Make post-apply readback host-aware, so an operation that succeeded but
  has not settled is not rescheduled forever.
- [x] Move `CountingProbe`, `transfer_session_probe` and `operation_probe`
  out of `longhorn-tauri-update`. Done 2026-08-09, and more thoroughly than
  planned: the whole crate had no Tauri code, so it was absorbed into
  `longhorn-update` rather than partially drained.
