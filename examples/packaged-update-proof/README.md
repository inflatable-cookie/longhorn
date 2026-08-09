# Packaged Update Proof

Replaces a **real** macOS application bundle with `longhorn-update-native`.
Card 162 evidence.

## Why it exists

`longhorn-update-native`'s own tests build synthetic archives — flat files at
mode `0644` under a fake `Example.app`. They prove the verification and the
traversal bounds, and they cannot prove that a real bundle survives the round
trip, because a real bundle has executable bits and, in general, symlinks.

## Running

Needs a packaged application. Any `.app` will do; Soundcheck is the one this
was proved against, because it is a real product that already composes five
`longhorn-tauri-*` crates.

```sh
cd ~/Dev/projects/soundcheck && cargo tauri build --debug --bundles app
cd ~/Dev/projects/longhorn && cargo run -p longhorn-packaged-update-proof -- \
  --app ~/Dev/projects/soundcheck/target/debug/bundle/macos/Soundcheck.app
```

It never touches the bundle you point it at. Both the "installed" application
and the update are copies in a temporary directory.

## What it claims

| Claim | Why it matters |
| --- | --- |
| a tampered artifact is rejected | `SignatureRejected` specifically, not a generic fault |
| a tampered artifact leaves the install untouched | refusing after damaging the install is not refusing |
| a verified artifact replaces the bundle | the `Info.plist` version changes on disk |
| executable bits survive the round trip | the thing synthetic `0644` fixtures cannot show |

Relaunch is **not** claimed. macOS separates replacement from relaunch and
`longhorn-update-native` keeps that separation rather than hiding it, so
relaunch belongs to the host. See contract 018.

## Recorded run

2026-08-09, macOS 25.5, against `Soundcheck.app` 0.1.0 → 0.1.1:

```json
{"outcome":"pass","bundle":"Soundcheck.app",
 "claims":{"aTamperedArtifactIsRejected":true,
           "aTamperedArtifactLeavesTheInstallUntouched":true,
           "aVerifiedArtifactReplacesTheBundle":true,
           "executableBitsSurviveTheRoundTrip":true},
 "executables":{"before":["Contents/MacOS/assistant-proof","Contents/MacOS/soundcheck-app"],
                "after":["Contents/MacOS/assistant-proof","Contents/MacOS/soundcheck-app"]},
 "versions":{"installed":"0.1.0","applied":"0.1.1","requested":"0.1.1"}}
```

## Not in `effigy qa`

It needs a packaged application, which the gate does not build. Same class as
`prototypes/gpui-windowing` — see the open papercut about heavyweight
out-of-gate evidence.
