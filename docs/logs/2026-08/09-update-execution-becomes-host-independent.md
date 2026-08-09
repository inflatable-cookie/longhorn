# Update Execution Becomes Host-Independent

Date: 2026-08-09
Card: 162 (batch 2)
Roadmap: g02.012 — closes the milestone

## Result

Contract 018 is amended: Longhorn owns update execution on every host, and
`longhorn-update-native` is the single implementation. This supersedes the
2026-08-08 amendment that made execution host-dependent.

Card 162 is complete. g02.012 is complete.

## Why the previous decision was wrong

It assumed the Tauri updater plugin could verify an artifact Longhorn hands
it. Read from `tauri-plugin-updater` 2.10.1, it cannot:

- `verify_signature` is called in exactly one place, at the end of
  `Update::download`.
- `Update::install(bytes)` reaches the platform installer with no
  verification of any kind.
- `Update`'s fields are private and only a network `check()` constructs one,
  so no adapter can wrap bytes it already holds in one.

An adapter therefore either surrenders the artifact to the plugin's own
downloader — a different contract — or hands the plugin unverified bytes,
which contract 018 forbids absolutely. There is no third option, which makes
"two implementations, one conformance suite" unsatisfiable rather than merely
unproven.

## Decisions

**Build for the host with no plugin; let the other inherit.** Operator's
reasoning, and it is the stronger form of the argument: an implementation
that satisfies GPUI necessarily satisfies Tauri, because nothing in it is
host-specific. The reverse ordering — build for the plugin, bolt on a
fallback — leaves the fallback under-exercised, and the fallback is exactly
where an unverified artifact would eventually get through.

**Tauri's updater stays the specification, not the mechanism.** Its artifact
shape — a gzip tar whose single top-level entry is the application — is what
Longhorn matches, so one signed release still serves both hosts. Longhorn
diverges only where copying would be unsafe: no shell interpolation,
classified failures, bounded extraction. That was already true of the
implementation; it is now also the contract's stated position.

**The conformance suite stays, with one implementation.** It could have been
deleted as redundant. It is what makes "verify before anything reaches disk"
a checked claim rather than a comment, and a Windows installer would have to
meet it.

## Evidence

`examples/packaged-update-proof` replaces a real macOS bundle. Run against
`Soundcheck.app` 0.1.0 → 0.1.1:

```json
{"outcome":"pass","claims":{"aTamperedArtifactIsRejected":true,
 "aTamperedArtifactLeavesTheInstallUntouched":true,
 "aVerifiedArtifactReplacesTheBundle":true,
 "executableBitsSurviveTheRoundTrip":true}}
```

The fourth claim is one the crate's own tests structurally cannot make: they
archive flat files at mode `0644` under a fake `Example.app`. A real bundle
carries executable bits, and an installer that dropped them would replace
cleanly and produce an application that will not start.

Soundcheck was chosen over a fresh proof application because it is a real
product already composing five `longhorn-tauri-*` crates. It needed no
changes — the proof consumes a build output and copies it twice, so the real
build is never at risk.

## What changed in code

Nothing structural. Five doc comments asserted the plugin story and were
corrected; `longhorn-tauri-update`'s "installation itself is the Tauri
updater plugin's job" had become actively false. `longhorn-update-native`'s
crate description no longer describes itself as a fallback.

## Open

Windows NSIS and MSI are unimplemented. The plugin is the obvious donor
specification when they are wanted, on the same terms as the macOS path.

`CountingProbe`, `transfer_session_probe` and `operation_probe` still sit in
`longhorn-tauri-update` and reference no Tauri — the leak class Card 161
closed for windowing. Not blocking; Card 162 held that surface until now.
