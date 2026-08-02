# Secondary-consumer Private Artifact Admission

Date: 2026-08-02
Roadmap: g01.016
Card: 114
State: complete; Card 115 ready

## Result

The exact Soundcheck-, Bovine-, and Jetstream-shaped private graphs pass from
produced artifacts. No consumer or Poodle repository changed. No package was
published, tagged, or released.

## Exact Sources And Sets

- Longhorn selected source:
  `ec465b2a86fe6fbaef789b3677a8e7288e7df2d2`
- Longhorn selected tree:
  `e5556c742104d8d613c0816ecf601f6fefbabc446e5aab22a3fc600ee138308c`
- Poodle source: `208532f0d18dcd1683cdef157e370d0ba0f0d3b3`
- Poodle set: `25083fe0c5f1b457572c5cb2eb3e3e88f06ed92f55a700d25a9f22d56492cc69`
- Longhorn TypeScript set:
  `7f62a7d21889c778803cda687248a9048e497cee80d2c2ceb7fa6957e18b3ce0`
- Longhorn Rust set:
  `42a1a400a7a6066614273a44d86c1686991c92c5454594efe1000aa1730d65c8`
- isolated Rust lock:
  `72d92e91730bab5691cbd6113358ce524b4ff39f9db76b0900bb9c2bc5853edf`

## Proof Shape

Fresh packing produced 5 Poodle tarballs, 9 Longhorn TypeScript tarballs, 18
selected Rust archives, and 3 dev-only support archives. Renderer graphs pass
at 7/4/6 selected Longhorn packages for Soundcheck/Bovine/Jetstream. Rust
graphs pass at 14/6/6 selected crates. Every non-selected package is absent
from its consumer graph.

All renderer locks resolve artifacts outside the Longhorn/Poodle workspaces,
contain no source alias, and resolve one Svelte runtime. All Rust checks use
Rust 1.85.0, Tauri 2.11.5, `--locked`, and `--offline`.

## Evidence

- `fixtures/migration/secondary-consumer-card114/private-artifact-admission-v1.json`
- `scripts/verify-secondary-consumer-private-artifacts.ts`
- `effigy proof:secondary-consumer-private-artifacts`

## Next

Execute Card 115 only. Migrate Soundcheck's stable product root, separate app
and window domains, external SQLite participant, and protected primary-window
host. Bovine and Jetstream remain read-only.
