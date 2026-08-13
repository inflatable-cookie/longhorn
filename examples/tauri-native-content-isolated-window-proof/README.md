# Tauri Native Content Isolated Window Proof

**Retired 2026-08-13 — evidence retained, source removed.** See Card 198.

The packaged application that produced this evidence is gone. It could not be
built (no icon, and no documented command built it), nothing invoked it, and
its findings were already recorded in `docs/logs/2026-08/`.

What stays is `evidence/`, and it is **not** inert:
`scripts/verify-native-content-artifacts.ts` reads `inventory.json` and the
recorded run on every gate, so the finding is still checked even though the
thing that produced it is not re-run.

That distinction is the point. A recorded run is an artifact; the harness that
produced it is a maintenance cost, and only one of the two was worth keeping.

To produce fresh evidence, the packaged proof would have to be rebuilt —
recover it from history at `examples/tauri-native-content-isolated-window-proof`.
