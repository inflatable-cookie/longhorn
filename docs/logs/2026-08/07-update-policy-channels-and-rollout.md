# Update Policy, Channels, And Rollout

Date: 2026-08-07
Card: 151
Roadmap: g02.009

## Result

`longhorn-update` decides whether to offer an update. It never fetches,
never verifies a signature, and never installs. Keeping verification out is
what allows the artifact host to be untrusted infrastructure, so it is a
boundary rather than a layering preference.

## Shape

- `Channel` (production, beta, nightly) with `stages_rollout` true only for
  production; `BuildIdentity` stamps channel and version into the
  diagnostics seam.
- `ChannelManifest` — version, notes, `minimum_version`, optional `rollout`,
  artifacts by target. Optional fields stay out of the wire form.
- `InstallId`, `RolloutFraction`, `Rollout::includes`.
- `CheckKind`, `OfferReason`, `UpdateAvailability`, `evaluate`.
- `Deferral` and `DeferralCause` (user postponed, work in flight,
  installation not writable), with `is_retryable` distinguishing "we will
  try again" from "here is how to do it yourself".
- Dependencies: `longhorn-core`, `semver`, `serde`, `sha2`. `semver` is new
  to the workspace; `sha2` was already there.

## Decisions

**Eligibility is SHA-256 over install id, a zero byte, and the seed**, read
as a `u64` divided by 2^64. The separator stops `("ab", "c")` colliding with
`("a", "bc")`. Dividing by 2^64 rather than `u64::MAX` keeps the position
strictly below 1.0, so a full rollout is genuinely everybody and a zero
rollout is genuinely nobody.

**Order in `evaluate` is load-bearing.** Up-to-date, then ahead-of-channel,
then the mandatory floor, then user-initiated bypass, then rollout. The
floor precedes rollout so a security release is never staged away, and that
ordering is asserted rather than left to reading.

**Ahead-of-channel is its own state.** An install on `1.3.0-nightly.4` that
selects production sits ahead of production `1.2.9` and receives nothing
until `1.3.0` ships. Correct, and indistinguishable from a broken updater
unless said out loud, so it is not folded into `UpToDate`.

**A deferral covers one version.** Declining `1.3.0` is not a standing
refusal of `1.3.1`; treating it as one would silently strand an install.

## Evidence

30 tests. The ones that carry weight:

- determinism across 100 repeated checks for the same install and release
- monotonic widening across 21 fractions × 300 installs: no offer already
  made is ever withdrawn, and a full rollout reaches everybody
- distribution — a half rollout lands 900-1100 of 2000 installs
- both overrides, and the boundary case that an install *at* the floor is
  still subject to rollout
- beta and nightly never stage
- prerelease rejoin falls out of semver ordering with no special handling

`cargo fmt --check` clean, clippy clean on both feature passes,
`cargo test --workspace --locked` 152 suites green.

## Notes

The rollout tests find an excluded install by search rather than hardcoding
one, so they keep testing what they claim to if the hash ever changes.

`docs/reference/api-surface.md` regenerated and
`docs/architecture/package-topology.md` updated for the new crate. Crate
count moves 38 → 39.
