# Rust Toolchain Floor

Date: 2026-08-04
Roadmap: g02 candidate runway (Tier A)

## Result

The declared minimum supported Rust version rises from 1.85 to 1.90,
releasing every dependency the old floor was holding back.

## Why

Nothing external required 1.85. No other repository in the portfolio
declared an MSRV, and the development toolchain is 1.96 — it was a
voluntary promise that cost zip 8.x and seven MSRV-pinned transitive
crates. The measured ceiling of everything wanted is 1.88; 1.90 clears it
with headroom while sitting roughly eleven months behind stable, so no
developer is forced onto a recent toolchain. Reviewed at each dependency
sweep, raised only when a dependency worth having demands it.

## Shape

- `rust-version` 1.85 → 1.90; contract 012's MSRV claim updated.
- 34 `cargo +1.85.0` gate invocations in `effigy.toml` and 15 artifact-proof
  scripts moved to `+1.90.0`. The scripts were the non-obvious half: they
  build crates in isolated workspaces on the pinned toolchain, so they, not
  the manifest, are what actually enforce the floor.
- zip restored to `=8.6.0`; darling, plist, serde_with, time,
  idna_adapter, writeable, and the icu family all released forward.
- Clippy gates `collapsible_if` on the declared MSRV, because its suggested
  fix — let-chains — stabilised in 1.88. Raising the floor surfaced the
  lint on pre-existing code; the nested `if let { if cond {` blocks are now
  let-chains. Mechanical, no semantic change.

## Portfolio

Applied the same floor to signal, underlay, poodle (14 standalone crates),
monkey (from 1.70), and swallowtail (from 1.93). Swallowtail keeps
`rust-version = "1.94.1"` on its Bedrock adapter, which its AWS SDK
genuinely requires — the uniform thing is the policy, not the number.
Jetstream and soundcheck-library were dirty and still need the one-line
addition.

## Evidence

- 149 workspace suites, Clippy, and fmt clean at the new floor
- history artifact proof rebuilds on `+1.90.0` from an offline lock
- current-facing docs updated; batch logs, the frozen Card 127 receipt
  reference, and the greenfield and consumer proof matrices deliberately
  keep 1.85, because that is what was proven at the time
