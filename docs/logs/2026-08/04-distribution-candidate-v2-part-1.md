# Distribution Candidate V2, Part 1

Date: 2026-08-04
Card: 149 (operator-held)
Roadmap: g02.008

## Result

The bridge optional-peer demotion parked behind the Card 127 receipt is
executed and proven; the candidate v2 verifier exists; the diagnostics
adoption guide is published. Receipt generation is gated.

## Shape

- `@longhorn/bridge` depends only on `@longhorn/core`; `@longhorn/tauri` is
  an optional peer reached through the `./tauri` and `./tauri-events`
  subpaths, following the `packages/operation` pattern. Asserted by the
  bridge package test, the bridge-topology artifact proof, the
  operation-notification artifact proof, and the five bridge proof
  consumers (which compile the shared `./tauri` subpath import and install
  the peer).
- `scripts/verify-private-candidate-card149.ts` +
  `scripts/private-candidate-card149/` carry the candidate surface with
  truthful 18-package/38-crate counts and the optional-peer expectation.
- `docs/guides/system-composition.md` gains the best-effort diagnostics
  adoption section (install-once sink, stable area labels, no-retry rule).

## Gate

The receipt freezes consumer graphs and asserts clean selected manifests;
nucleus's `apps/desktop/src-tauri/Cargo.toml` is dirty under the active
nucleus integration thread (read-only from here by working posture).
Regenerate with `bun scripts/verify-private-candidate-card149.ts --write`
after nucleus settles, then wire effigy tasks, supersede the Card 127
receipt, refresh the reference doc and CHANGELOG, and run full QA.
