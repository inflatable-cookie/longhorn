# 231 Agent Control Capture

Status: done 2026-08-19
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

- [x] `screenshot` returns a current-DOM PNG for frontmost, unfocused,
      occluded, and minimized windows of the packaged app, evidence
      recorded like Card 227's matrix
- [x] capture requires no permission prompt and no private API in the
      packaged bundle
- [x] failures surface as typed core-vocabulary errors, never a panic on
      the main thread
- [x] non-macOS targets compile and answer typed `Unsupported`
- [x] milestone g02.031 acceptance holds: release artifact scan still
      clean with capture code behind the same feature
- [x] `effigy qa` passes

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

## Closeout

Status: done 2026-08-19, same branch and worktree as Card 230.

**Capture path.** `crates/longhorn-tauri-agent-control/src/capture.rs`
keeps the spike's proven mechanics — `with_webview` main-thread dispatch,
retained `WKWebView`, `RcBlock` completion → tokio oneshot — with every
donor shortcut removed: no `expect`, every failure a typed
core-vocabulary error (`EvaluationFailed` for JS, `Unsupported` with the
reason for capture-side failures, the vocabulary's closest fit), PNG
encoded via TIFF → `NSBitmapImageRep`. Public API only: nil-configuration
`takeSnapshotWithConfiguration:` (current viewport). Per-window targeting
runs through Card 230's window scope. The objc2 line (objc2 0.6.4,
objc2-app-kit/foundation/web-kit 0.3.2, block2 0.6.2) matches the spike's
versions and was already resolved in the workspace lock through wry — no
pin moved; the workspace's tauri 2.11.5 / tauri-runtime 2.11.3 pair
serves `with_webview` unchanged. The workspace's `unsafe_code = forbid`
is crate-scoped to `deny` for this crate with the single `allow` on the
capture module; the three genuinely-`unsafe fn` calls each carry a SAFETY
comment.

**`evaluate` in this lane.** The handoff listed `evaluate` among the
tools answering `Unsupported` here; the card's matrix judgment requires
"`evaluate` bracketing exactly as the spike did". Resolved in the card's
favor: `evaluate` is implemented as the raw `evaluateJavaScript` escape
hatch through the same capture bridge (donor mechanics, typed errors) —
host mechanics, not the g02.032 TS shim, which still owns the semantic
tools (`snapshot`, input dispatch, `wait_for`, ref resolution).

**Packaged freshness matrix.** New proof composition
`examples/agent-control-proof` (workspace member): a minimal Tauri app
mounting the plugin behind `dev`, its contract-006 registry carrying
`proof:ping`, `proof:window.minimize`, `proof:window.restore` so window
state is scripted through the same `command` tool an agent uses. The
committed driver `freshness-matrix.ts` (the spike's uncommitted probe was
a Card 227 review note) builds the `.app` via `bunx @tauri-apps/cli
build`, launches it, reads the discovery file, and probes each state with
`evaluate`-bracketed screenshots. Freshness judgment is the spike's,
automated: the page encodes its counter in the background hue with a 47°
stride, and the captured center-left pixel must match a bracketed
counter's hue — DOM-relative, never wall-clock.

Run 2026-08-19 on the operator's display
(`examples/agent-control-proof/evidence/2026-08-19T17-41-52-packaged/`,
PNGs + `matrix.json`, schema
`longhorn.agent-control-freshness-matrix.v1`):

| state | bracket | matched counter | fresh |
| --- | --- | --- | --- |
| frontmost | 1..1 | 1 | yes |
| unfocused | 3..3 | 3 | yes |
| occluded (Terminal window over the app) | 5..5 | 5 | yes |
| minimized | 6..6 | 6 | yes |
| restored | 8..8 | 8 | yes |

No divergence from the spike's debug-binary results: every state fresh
against the DOM, no permission prompt, no entitlement, no private API.
Neither stop condition triggered. Clean quit removed the discovery file
(`discoveryRemovedOnQuit: true`), re-proving the lifecycle on a real
bundle.

**Platform honesty.** macOS is the only proof platform; the cfg split
leaves non-macOS hosts two trivial branches answering typed
`Unsupported`. Only the macOS toolchain ran here (no Linux/Windows
target installed on this host, and a `wry` cross-check needs system
libraries this host cannot provide) — the non-macOS compile claim rests
on the cfg construction, recorded rather than overclaimed.

**Found while proving (composition contract note).** A macOS quit
delivers `RunEvent::Exit` without a preceding `ExitRequested`; an app
hooking only `ExitRequested` strands the discovery file on clean quit.
The mount docs and the proof app hook both events.

**Versions (workspace lock):** tauri 2.11.5, tauri-runtime 2.11.3, objc2
0.6.4, objc2-app-kit 0.3.2, objc2-foundation 0.3.2, objc2-web-kit 0.3.2,
block2 0.6.2.

## Continuation

g02.031 closes with this card. g02.032 (TS shim, semantic tools, end-to-end
proof) compiles next; its cards are reserved as 232-234.
