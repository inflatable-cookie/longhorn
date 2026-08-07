# 153 Restart Interlock And Tauri Install

Status: ready
Owner: Tom
Roadmap: g02.009 batch 3
Governing refs: contracts 018 and 017; research memo 019
Depends on: Card 151
Auto-start next card: no

## Objective

Build `longhorn-tauri-update`: obtain a quiescence receipt from the
lifecycle coordinator before any install, then hand the chosen artifact to
the Tauri updater plugin for download, verification, and replacement.

## Rationale

This is the only part of the milestone a consuming application could not
write for itself. Longhorn knows what is in flight — pending flushes,
uncommitted transfer sessions, live async operations. An install that
relaunches during a transfer commit is data loss.

## Scope

- restart-readiness contract against the lifecycle coordinator
- Tauri updater plugin wiring
- the two open mechanism questions below, settled before building

## Steps

1. **Settle whether Tauri installs a specifically chosen artifact**, or only
   what its configured endpoint returns. If endpoint-only, serve the
   resolved manifest over a loopback endpoint bound to `127.0.0.1` with a
   one-shot nonce. Signature verification stays inside the plugin either
   way, so this is a crate-shape decision, not a security one. Record the
   finding before writing the wiring.
2. **Settle how `installMode` and the macOS in-place bundle replacement
   interact with Longhorn's teardown ordering.** Record it.
3. Define the quiescence receipt: pending flushes, uncommitted transfer
   sessions, in-flight async operations. Reuse the existing teardown and
   `shutdown_flush` machinery from contract 017 rather than adding a
   parallel notion of "busy".
4. Refuse-and-defer on non-quiescence, carrying the reason. A refused
   restart is never a cancelled one.
5. Handle non-writable installations — Homebrew casks, administrator-
   installed copies — with a manual-download fallback rather than an error.
6. Never implement, wrap, or bypass signature verification.
7. Tests: install blocked by each quiescence condition, deferral carries its
   reason, install proceeds once quiescent, non-writable fallback.

## Acceptance Criteria

- no install proceeds while any covered work is in flight
- the interlock reuses contract 017 machinery rather than duplicating it
- both mechanism questions are recorded with findings before wiring lands
- verification remains entirely inside the Tauri plugin
- workspace QA passes

## Evidence Required

- the two recorded mechanism findings
- per-condition interlock tests
- the non-writable-installation fallback path

## Stop Conditions

- the lifecycle coordinator cannot express quiescence without a public API
  break for existing consumers

## Next Task

Card 154.
