# Update Crate Names Follow The Dependency

Date: 2026-08-09
Card: 162 follow-up
Roadmap: g02.012

## Result

Two update crates renamed, one of them out of existence.

- `longhorn-tauri-update` is **absorbed into `longhorn-update`**. It had no
  Tauri code and no Tauri dependency.
- `longhorn-update-native` is **`longhorn-update-install`**. `-native` implied
  a fallback to a plugin path contract 018 no longer has.

The update tier is now two crates: `longhorn-update` (pure policy — channels,
comparison, rollout, deferral, and the restart interlock) and
`longhorn-update-install` (verification, extraction, atomic replacement).

## Why the prefix was wrong

`longhorn-tauri-update`'s own crate documentation said it was "named `tauri-*`
by role rather than dependency". That is a rationalisation, not a rule: every
other `tauri-*` crate in the workspace carries a real `tauri` dependency, so
the prefix means something, and one crate opting out of that meaning makes the
whole convention unreadable. Its contents — `UpdateGate` and three quiescence
probes — were pure policy over pure traits.

Recorded twice before as a leak worth fixing: Card 161's scan of the same
class, and Card 163's finding that the probes "sit in `longhorn-tauri-update`
and reference no Tauri". Both times it was deferred because that crate was
Card 162's live surface. Card 162 closed, so it was done — and more
thoroughly than planned, because draining the probes would have left a crate
containing one pure gate under a prefix that still lied.

## Why not merge everything into `longhorn-update`

Because `longhorn-update` is pure, and that is load-bearing rather than
decorative. Its dependencies are `core, semver, serde, sha2`: no filesystem,
no network, no clock. `longhorn-update-install` pulls `minisign-verify`,
`tar` and `flate2` and writes to disk.

Merging execution into policy would trade a property that lets policy be
composed and tested anywhere for a tidier crate count. The policy/execution
split is the same one `longhorn-windowing` keeps against its host adapters.
The interlock merged *in* because it decides rather than acts; the installer
stayed *out* because it acts.

## Evidence

- 63 tests across the two crates, all passing
- `examples/packaged-update-proof` re-run against the real `Soundcheck.app`
  after the rename: still `"outcome":"pass"` on all four claims
- no external consumer referenced either crate, so nothing outside the
  workspace moved
