# Restart Interlock Host

Date: 2026-08-07
Card: 153 (batch 2)
Roadmap: g02.009

## Result

`longhorn-tauri-update` carries the concrete quiescence probes and the
install gate. 10 tests. The concrete `tauri-plugin-updater` installer sits
behind an injected port and lands with Card 159.

## Shape

- `CountingProbe` plus `transfer_session_probe` and `operation_probe`,
  wrapping the counts the real hosts already expose
  (`TransferCoordinator::session_count`, an operation authority's active
  records).
- `UpdateGate` — quiesce, install, relaunch, in that order.
- `UpdateInstaller` and `InstallError` as the injected port.

## Decisions

**Probes read their count at probe time, not at construction.** The whole
value of an interlock is that it answers about *now*; a probe that captured
a count when it was built would authorise a restart against a stale reading.
Tested by mutating the count between two probe calls.

**Quiescence is rechecked at install time rather than reused from the
offer.** The user may start a transfer between being offered an update and
accepting it. Also tested directly, because reusing the earlier receipt is
the natural implementation.

**A failed relaunch is `InstalledAwaitingRelaunch`, never a failure.** The
update reached disk. Reporting it as a failure would be false and would
invite the user to retry an update they already have — which matters
specifically because tauri#11392 says macOS relaunch does fail in the wild.

**A non-writable installation gets its own deferral cause**, distinct from
transient work in flight, because it cannot resolve itself and the remedy is
a manual download.

**The installer is an injected port**, for the same reason the licence
credential store is: the interlock is the part only Longhorn can write and
it is fully testable behind the port, whereas the concrete plugin-backed
installer cannot be exercised headlessly at all. Shipping an unverified one
in the library would add nothing and would put the least trustworthy part of
the update path behind a green check.

## Evidence

10 tests. Beyond the two above: an open transfer session refuses the install
entirely rather than warning; the deferral reason names everything
outstanding, not the first probe to answer; the non-writable cause is not
retryable.

`cargo fmt --check` clean, clippy clean on both feature passes, full
workspace suite green.

## Notes

Crate count 40 → 41. `docs/reference/api-surface.md` regenerated and
`docs/architecture/package-topology.md` updated.
