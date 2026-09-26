# Contracts

One file per durable rule or interface. Each states the current rule, its
owner, and what depends on it.

- [Contract index](contract-index.md) — positioning, tiers, and the numbered
  contract register. Add each new contract there.
- [Release](release.md) — how Longhorn releases.
- [Writing style](writing-style.md) — the internal writing policy.
- Rust and TypeScript quality profiles and their deviations:
  `rust-quality-profile.json`, `rust-quality-deviations.json`,
  `typescript-quality-profile.json`, `typescript-quality-deviations.json`.

Work stops when the boundary it needs is still pending in the index.
