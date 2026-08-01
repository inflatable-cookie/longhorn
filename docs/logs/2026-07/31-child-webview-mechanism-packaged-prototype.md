# Child-webview Mechanism Packaged Prototype

Date: 2026-07-31
Card: 083
Roadmap: g01.013

## Result

Implemented the private child-view adapter over the Card 082 coordination
prototype and Tauri 2.10.3. The unstable `WebviewBuilder` and
`Window::add_child` calls live behind one narrow runtime port. The nested
workspace is non-publishable and creates no production authority.

The packaged macOS 26.5.2 arm64 run passes with one explicit environment gap:
the host exposed one 2x monitor, so no native scale transition was available.
That claim remains unmet and unsimulated.

## Native Matrix

- controlled remote WebKit content creates and reaches ready
- semantic viewport converges to fresh native 2x physical bounds
- host resize, viewport move, zero-size bounds, and restore converge
- inactive, overlay, and drag inhibitors arrive as explicit consumer reasons
- six hide/show probes preserve one JavaScript session and increasing counter
- renderer unmount does not close or forget the native child
- focus request applies; portable child-focus readback remains unknown
- portable effective visibility readback remains unknown
- explicit close records `DetachStarted`, `Detached`, `Detaching`, then `Absent`
- generation-two replacement rejects a generation-one callback
- host destruction invalidates generation two before a late callback

The first packaged trace caught the missing `Attached -> Detaching -> Absent`
integration evidence. The adapter now emits close-start before the native call
and close-complete after it. The coordinator admits that recorded chronology;
its lifecycle contract was not relaxed.

## Security

The content URL, origin admission, data-store identity, popup policy, download
policy, and remote-capability posture are constructor inputs. The local hidden
controller has an empty permission set. No capability matches the remote
child and global Tauri injection is disabled.

The packaged run observes denied cross-origin navigation and denied download.
WebKit did not produce a new-window callback for the scripted target-blank
probe, so packaged popup-hook observation is absent. The adapter's closed deny
hook and capability structure remain covered by fake-runtime and config tests.

## Boundary

The adapter depends directly on `longhorn-core`, the private Card 082
prototype, `serde`, and Tauri. It imports no isolated-window, plugin, GPU,
Svelte, or Poodle adapter. Raw `tauri::Webview` handles stay inside the runtime
port. Windows and Linux remain unproved.

The packaged `.app` is 9,192 KiB. Its executable SHA-256 is
`abc6eac90be904d72ce75b2c43159390eb71a120c91fa6d71ee9664b4b650086`.
The committed report, transcript, target ledger, and inventory preserve the
proof without committing build output.

## Validation

- six deterministic adapter contract tests pass on Rust 1.85
- nested workspace check, strict Clippy, and formatting pass
- Tauri release packaging produces the macOS `.app`
- the clean scripted transcript contains no fixture or proof failure
- focused Effigy evidence verification passes
- the full repository `effigy qa` gate passes

## Next Task

Execute Card 084. Prove isolated native-window content-size negotiation,
helper loss, and bounded teardown with a controllable fake child. Keep plugin,
child-webview, and backing-surface authority out of that graph.
