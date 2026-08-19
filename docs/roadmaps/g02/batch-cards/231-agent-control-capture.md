# 231 Agent Control Capture

Status: ready
Owner: Longhorn maintainers
Roadmap: g02.031
Governing refs: contract 022; contracts 010, 012, 020; memo 024;
Card 227 evidence (`prototypes/agent-control/evidence/`)
Depends on: Card 230
Auto-start next card: no — g02.032 is a separate dispatch

## Objective

`screenshot` works through the plugin on macOS — fresh for unfocused,
occluded, and minimized windows — proved against a packaged app bundle,
not just `cargo run`.

## Scope

- **Capture path.** `WKWebView.takeSnapshot` via `with_webview`, following
  the Card 227 spike's proven mechanics (main-thread dispatch, retained
  webview, completion → oneshot) but production-shaped: typed errors
  through the core vocabulary instead of `expect`, PNG encoding, and
  per-window targeting through Card 230's window scope. The spike's
  version pin intel applies (tauri-runtime 2.10.1 with tauri 2.10.3 —
  confirm or move deliberately).
- **Freshness proof, packaged.** Re-run the Card 227 freshness matrix —
  frontmost, unfocused-visible, fully occluded, minimized — against a
  packaged `.app` composed from the plugin (extend an existing
  `examples/` proof composition or add one), judged DOM-relative by
  `evaluate` bracketing exactly as the spike did. Record results in the
  card; divergence from the spike's debug-binary results is a finding,
  not a failure.
- **Platform honesty.** macOS is the proof platform. Windows/Linux capture
  is not implemented here: those hosts answer typed `Unsupported`, and the
  limitation is recorded where contract 022's per-host evidence lives
  (contract 020 discipline: a claim proved on one backend does not close a
  host-tier contract).

## Acceptance Criteria

- [ ] `screenshot` returns a current-DOM PNG for frontmost, unfocused,
      occluded, and minimized windows of the packaged app, evidence
      recorded like Card 227's matrix
- [ ] capture requires no permission prompt and no private API in the
      packaged bundle
- [ ] failures surface as typed core-vocabulary errors, never a panic on
      the main thread
- [ ] non-macOS targets compile and answer typed `Unsupported`
- [ ] milestone g02.031 acceptance holds: release artifact scan still
      clean with capture code behind the same feature
- [ ] `effigy qa` passes

## Validation

`effigy qa`; the packaged freshness matrix run on a macOS host; the
Card 230 artifact scan re-run feature-off.

## Stop Conditions

- packaged capture behaves differently from the spike's debug binary in a
  way that weakens contract 022's freshness claim (e.g. stale occluded
  snapshots under App Sandbox or hardened runtime) — stop, record the
  matrix, and bring it back: that amends the contract before g02.032
  builds on it;
- capture needs an entitlement or permission prompt in the packaged
  bundle — same treatment.

## Continuation

g02.031 closes with this card. g02.032 (TS shim, semantic tools, end-to-end
proof) compiles next; its cards are reserved as 232-234.
