# 153 Restart Interlock And Tauri Install

Status: in progress — interlock complete, concrete installer awaits Card 159
Owner: Tom
Roadmap: g02.009 batch 3
Governing refs: contracts 018 and 017; research memo 019
Depends on: Card 151
Auto-start next card: no

## Objective

Build `longhorn-tauri-update`: obtain a quiescence receipt from the
lifecycle coordinator before any install, then hand the chosen artifact to
the Tauri updater plugin for download, verification, and replacement.

## Rationale

This is the only part of the milestone a consuming application could not
write for itself. Longhorn knows what is in flight — pending flushes,
uncommitted transfer sessions, live async operations. An install that
relaunches during a transfer commit is data loss.

## Scope

- restart-readiness contract against the lifecycle coordinator
- Tauri updater plugin wiring
- the two open mechanism questions below, settled before building

## Steps

1. **Settle whether Tauri installs a specifically chosen artifact**, or only
   what its configured endpoint returns. If endpoint-only, serve the
   resolved manifest over a loopback endpoint bound to `127.0.0.1` with a
   one-shot nonce. Signature verification stays inside the plugin either
   way, so this is a crate-shape decision, not a security one. Record the
   finding before writing the wiring.
2. **Settle how `installMode` and the macOS in-place bundle replacement
   interact with Longhorn's teardown ordering.** Record it.
3. Define the quiescence receipt: pending flushes, uncommitted transfer
   sessions, in-flight async operations. Reuse the existing teardown and
   `shutdown_flush` machinery from contract 017 rather than adding a
   parallel notion of "busy".
4. Refuse-and-defer on non-quiescence, carrying the reason. A refused
   restart is never a cancelled one.
5. Handle non-writable installations — Homebrew casks, administrator-
   installed copies — with a manual-download fallback rather than an error.
6. Never implement, wrap, or bypass signature verification.
7. Tests: install blocked by each quiescence condition, deferral carries its
   reason, install proceeds once quiescent, non-writable fallback.

## Acceptance Criteria

- no install proceeds while any covered work is in flight
- the interlock reuses contract 017 machinery rather than duplicating it
- both mechanism questions are recorded with findings before wiring lands
- verification remains entirely inside the Tauri plugin
- workspace QA passes

## Evidence Required

- the two recorded mechanism findings
- per-condition interlock tests
- the non-writable-installation fallback path

## Stop Conditions

- the lifecycle coordinator cannot express quiescence without a public API
  break for existing consumers

## Mechanism Findings — 2026-08-07

Both questions settled against `tauri-plugin-updater` v2 source before any
wiring, as the card required.

### 1. Installation is endpoint-only, but the endpoint is ours to choose

`Update` has private `extract_path` and `context` fields and **no public
constructor**. The only way to obtain one is `Updater::check()`, which
fetches from the configured endpoints. So Tauri will not install an
artifact we selected out of band.

Two builder methods make that a much smaller constraint than it reads as:

- `UpdaterBuilder::endpoints(Vec<Url>) -> Result<Self>` is settable at
  **runtime**, so the active channel picks the endpoint. No build-time fork,
  no loopback needed for channel selection.
- `UpdaterBuilder::version_comparator<F: Fn(Version, RemoteRelease) -> bool>`
  lets `longhorn-update::evaluate` make the decision, and `Update::raw_json`
  exposes response fields the plugin does not model. Rollout, channel, and
  `minimumVersion` can therefore ride inside the manifest the plugin already
  fetches.

The loopback shim is consequently **not** needed for the static hosts, only
for adapters whose manifest cannot be fetched by the plugin directly — the
private-GitHub asset-ID flow being the case in point. Card 152 owns that.

The constraint that remains: the endpoint must serve Tauri's own JSON shape,
so the manifest carries Longhorn's policy fields as extra keys rather than
being an arbitrary format. Contract 018 is unaffected.

### 2. macOS separates install from relaunch, which is what the interlock needs

- `installMode` is **Windows-only**. macOS has no equivalent knob.
- macOS `install` decompresses the `.tar.gz`, moves the current bundle to a
  backup path, moves the new one into place (escalating via AppleScript when
  the location needs it), and returns `Ok(())` **without relaunching**.

That separation is favourable: quiesce, install, tear down, relaunch are
four steps we order ourselves rather than one opaque call.

- `Update::install(&self, bytes: impl AsRef<[u8]>) -> Result<()>` is
  synchronous and separate from `download`, so the interlock can gate the
  install specifically rather than the whole download-and-install.
- **Known hazard, tauri#11392:** on macOS the app can fail to relaunch after
  install. A contributing factor in the report is a window-close handler
  calling `api.prevent_close()`. Longhorn's lifecycle host owns close
  handling, so this is our problem to get right, not an upstream curiosity
  to note. Linked PR tauri#12313.

## Progress

Batch 1 of this card is complete: the restart-readiness contract lands in
`longhorn-update` as `QuiescenceKind`, `OutstandingWork`, `QuiescenceProbe`,
and `QuiescenceReceipt`, with `as_deferral_cause` bridging to the deferral
model from Card 151. Pure and fully tested.

Every probe runs rather than short-circuiting at the first outstanding item:
a receipt that stopped early would report a reason that depends on probe
order, and would understate what the user is about to interrupt.

Batch 2 is now complete too. `longhorn-tauri-update` carries the concrete
probes (`CountingProbe`, `transfer_session_probe`, `operation_probe`, read at
probe time so a stale receipt cannot authorise a restart) and `UpdateGate`,
which orders quiesce, install, relaunch and is fully tested behind an
injected `UpdateInstaller`.

The port is injected for the same reason the licence credential store is:
the interlock is the part only Longhorn can write and it is testable behind
the port, whereas the concrete plugin-backed installer cannot be exercised
headlessly at all. It lands with Card 159, where macOS bundle replacement
and the tauri#11392 relaunch path can be proved rather than assumed.

Two behaviours the tests pin down that a naive implementation gets wrong:
quiescence is rechecked at install time rather than reused from the offer,
because the user may start a transfer in between; and a failed relaunch is
`InstalledAwaitingRelaunch`, never a failure, because the update reached
disk and reporting otherwise invites retrying an update already installed.

Outstanding: the concrete `tauri-plugin-updater` installer, under Card 159.

## Next Task

Finish `longhorn-tauri-update` against a packaged proof application, then
Card 154.
