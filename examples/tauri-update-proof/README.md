# Tauri Update Proof

This [workspace-member app](src-tauri/Cargo.toml) exercises restart admission with a real transfer
session and relaunch in a packaged macOS Tauri host. It wires
`longhorn-update::UpdateGate` directly to `longhorn-transfer`.
[longhorn-tauri-update](../../crates/longhorn-tauri-update/src/lib.rs) supplies
the reusable Tauri update adapter for applications. The
[packaged update proof](../packaged-update-proof/README.md) covers staged
bytes, signature verification and bundle replacement.

## Transfer interlock

Open a transfer session, attempt an install, then close the session and try
again. The controls report whether the gate would authorize installation.
The gate reads `TransferCoordinator::session_count()` through
`transfer_session_probe`. A lease only advertises a drop target. The
interlock counts sessions opened through `create_session` with a real
`TransferSourceAuthority`. The headless tests assert that the coordinator
accepted and counted the session, that admission reports a cause when it
refuses, and that a quiescent host authorizes installation.

## Packaged relaunch

The app installs `api.prevent_close()` on every close request to reproduce the
close handling relevant to tauri#11392. **Request relaunch** writes a marker
to the app data directory, then calls `request_restart` so Tauri sends
`ExitRequested` and `Exit`. After relaunch the app reads and clears the
marker and reports `relaunchClaim: met`. If the app stays closed, the marker
records the attempted restart.

From the repository root, clean the proof package if frontend assets have
changed since a previous build. The build script does not track them as
rerun inputs.

```sh
oxide clean -p longhorn-tauri-update-proof
cd examples/tauri-update-proof/src-tauri && oxide exec -- cargo tauri build
```

Run the build inside the proof's `src-tauri` directory to select its bundle
configuration. From the repository root, launch the generated
`target/release/bundle/macos/Longhorn Update Proof.app`. Launch its executable
directly to keep stdout attached when diagnosing restart:

```sh
"target/release/bundle/macos/Longhorn Update Proof.app/Contents/MacOS/longhorn-tauri-update-proof"
```

Use the packaged app for the relaunch check. A dev run restarts its dev
binary and cannot establish packaged behavior.

## Recorded macOS result

On 2026-08-13, the packaged app on macOS 26.5.2 (25F84), arm64, with Tauri
2.11.5, reported one open session, denied admission with
`WorkInFlight { detail: "1 open transfer session" }`. It relaunched through
`request_restart` with the preventing close handler installed, then recovered
the marker. The result covers that host, version and restart entry point.
The headless tests cover transfer admission, and repeating the relaunch
claim requires the packaged app.
