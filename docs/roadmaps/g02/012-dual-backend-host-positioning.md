# g02.012 Dual-backend Host Positioning

Status: ready
Owner: Tom
Updated: 2026-08-08
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

- [ ] [Card 161](batch-cards/161-host-tier-positioning-and-leak-fixes.md)
  restates positioning, tiers the contracts, and closes the two leaks

### Batch 2. Delegated capability

- [ ] [Card 162](batch-cards/162-native-update-execution.md) builds the
  native update installer for hosts with no plugin, under one shared
  conformance suite

### Batch 3. Second backend

- [ ] [Card 163](batch-cards/163-gpui-host-adapter.md) implements the
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

- [ ] no pure crate depends on a host adapter
- [ ] no host-shaped concept sits unqualified in `longhorn-core`
- [ ] contract tiers are stated, and webview-edge contracts are optional
- [ ] update installs on a host with no plugin, under the same contract
- [ ] one host-contract claim is proved against both backends

## Acceptance Criteria

- [ ] `longhorn-windowing-config` compiles with no `longhorn-tauri-*`
  dependency
- [ ] every host-contract requirement is stated without naming a backend
- [ ] the native installer and the Tauri plugin path share one conformance
  suite
- [ ] the GPUI adapter's unproven claims are recorded as unproven

## Explicit Non-goals

- migrating Loophole or Nucleus. Both convert later; neither is near-term.
- deprecating Tauri. It stays first-class permanently: the fast path for
  prototyping and for applications that never justify conversion.
- a lowest-common-denominator host abstraction. Differences are stated, not
  erased.
- porting `packages/*`. A GPUI application composes a projection tier, not a
  parallel client tier.

## Next Task

Card 161. It is documentation plus a type move, unblocks the rest, and makes
the architecture honest about what it already is.
