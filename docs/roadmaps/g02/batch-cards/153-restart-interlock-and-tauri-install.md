# 153 Restart Interlock And Tauri Install

Status: in progress — batch 1 complete, host wiring outstanding
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

Outstanding: `longhorn-tauri-update` itself — probe implementations against
the live host, plugin wiring, and the relaunch path. Held back deliberately
rather than landed untested: the install and relaunch behaviour cannot be
exercised headlessly, and tauri#11392 means the relaunch path specifically
needs a real packaged application to verify against. Landing unverified
wiring would put the least trustworthy part of the update path behind a
green check.

## Next Task

Finish `longhorn-tauri-update` against a packaged proof application, then
Card 154.
