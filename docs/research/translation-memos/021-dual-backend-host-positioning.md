# 021 Dual-backend Host Positioning

Status: complete and promoted
Owner: Tom
Updated: 2026-08-08
Promotes: contract 020; the contract tiering in `contract-index.md`; the
update-execution reversal in contract 018; the g02.012 milestone.

## Prompt

Longhorn is positioned as a framework for Tauri applications. Poodle's GPUI
implementation makes a fully native Rust application viable, and
performance-sensitive products — Loophole, Nucleus — will want it. Decide
what Longhorn must become to support two first-class hosts rather than one,
without rewriting the applications that stay on Tauri.

## Sources

Workspace measurement at `1885ad64`: crate manifests, dependency edges,
`use tauri::` imports across all 41 crates, and the delegation surface named
in contracts 018 and 019. Poodle's repository checked for the reverse
dependency edge.

## Findings

### The separation already largely exists

**12.5% of the Rust is Tauri-bound** — 13,785 lines across 11 `tauri-*`
crates, against 96,569 pure. The domain crates were built framework-neutral
from g01, and the split holds under measurement rather than only in
description.

The Tauri surface actually touched is narrow: `AppHandle`, `Runtime`,
`WebviewWindow`, `Emitter`, `Manager`, `State`, `Monitor`, `WindowEvent`,
`LogicalPosition`/`LogicalSize`, `Url`. **No Tauri plugin appears in any
Longhorn manifest.**

`longhorn-bridge` already models a non-Tauri host: `BridgeHostForm` carries
`Direct` — "the client invokes an in-process host directly" — beside
`TauriLocal`, `LocalService`, `Remote`, and `LocalFirst`.

### Exactly one dependency leak, and it is a misplacement

`longhorn-windowing-config` imports `CapturedWindowPlacement`,
`WindowFlushRequest`, `WindowPlacementFlushTicket`, and
`WindowPlacementSink` from `longhorn-tauri-windowing`. **None of the four
references Tauri.** `WindowPlacementSink` is a plain trait;
`WindowPlacementFlushTicket` wraps a channel receiver. They are pure types
living in the wrong crate, and moving them to `longhorn-windowing` removes
the only pure-to-Tauri edge in the workspace.

### One conceptual leak, in the most expensive place

`longhorn-core` defines `ClientCssPx` — "webview content coordinate measured
in CSS pixels". CSS pixels are a webview concept; GPUI has logical and
physical pixels with a scale factor. It sits in the foundational crate, so
it propagates everywhere, and renaming after publication is a breaking
change across every consumer.

### Three kinds of Tauri dependence, and only one is real work

**Value boundaries — work unchanged.** More than expected.
`longhorn-tauri-config`'s `TauriDirectorySnapshot` is an enum of `PathBuf`s:
the caller obtains paths from Tauri and hands them in. A GPUI application
supplies the same paths from `directories` and reuses `longhorn-config`
entirely. The same holds for the injected `ports.rs` authorities, the
`CredentialStore` seam, the diagnostics seam, update policy, and the restart
interlock.

**Webview-only — moot, not ported.** The four native-content crates exist
because a webview cannot host native content; in GPUI everything is native.
The bridge exists because IPC crosses a process. `packages/*` is the far
side of that boundary. None of it needs an adapter; a GPUI application does
not compose it.

**Delegated capability — the real gap.** Functionality Longhorn never
implemented because Tauri did:

- **update execution** — download, minisign verification, unpack, bundle
  replacement, relaunch. Contract 018 states this belongs to the plugin.
- **system browser launch** — contract 019's RFC 8252 flow needs an opener.
- **OS notification delivery** — already Tier B deferred; neither host has
  it, so not a regression.

### The IPC tier is the largest thing that disappears

Of ~24,000 lines of TypeScript source, roughly 18,000 is boundary machinery:
6,023 generated protocol, 5,330 validators, and transport/client wiring.
What survives conceptually is the Svelte bindings (~2,000) and the Poodle
projections (~1,250) — about 3,000 lines of equivalent, in Rust, and simpler
because nothing serializes.

A GPUI host therefore needs a projection tier, not a parallel of
`packages/*`.

### The dependency direction runs Longhorn to Poodle

`@inflatable-cookie/longhorn-poodle` is a Longhorn package depending on
`longhorn-layout` and `longhorn-svelte`, peering on `@poodle/svelte`. The
poodle repository contains no reference to longhorn. Poodle is a generic UI
kit; Longhorn ships the projection that drives it. The GPUI stack repeats
the shape with `poodle-gpui` in place of `@poodle/svelte`.

Poodle has already proved a contract can carry two implementations — its
GPUI suite matches its Svelte suite against shared contracts. That is the
model for Longhorn's host contract, and the precedent that makes this
credible rather than speculative.

## Decision

Operator decisions recorded 2026-08-08:

1. **Two first-class hosts, permanently.** Tauri is not legacy. It remains
   the fast path for prototyping and for applications that never justify
   conversion. GPUI serves performance-sensitive products.
2. **Update execution: native implementation for non-Tauri hosts, the
   plugin for Tauri, one shared behavioural contract.** This reverses the
   authorize-only decision recorded in Card 153, which was correct while
   Tauri was the only host and becomes wrong the moment a second host has no
   plugin. The alternative — every consuming application implementing
   minisign verification and macOS bundle replacement — is exactly what a
   framework exists to prevent, and it is security-sensitive code to
   duplicate per product.
3. **Poodle remains the UI layer for both hosts.**

Compile contract 020 (host adapter), tier the contract register, amend
contract 018, and open g02.012.

## Open Questions And Planning Gaps

- **Naming.** `longhorn-tauri-*` sets the host-adapter convention, so
  `longhorn-gpui-*` follows. `longhorn-poodle` (TypeScript) carries no
  prefix, so its Rust sibling needs a deliberate name.
- **Proof coverage.** The first GPUI target is a small audio-conversion
  application. It exercises config, settings, operations, notifications,
  licence and update — and, being a product replacing a subscription, it
  exercises exactly the two delegated-capability gaps. It will **not**
  exercise multi-window placement, cross-window transfer, or lifecycle
  teardown under load, which are where Tauri's assumptions are most likely
  to have leaked. The host contract must not be declared complete on its
  evidence.
- Loophole and Nucleus convert later; neither is near-term.

## Consumer Exposure

None immediate. The leak fixes are internal; the tiering is documentation;
the new crates are additive and inert until composed. Existing Tauri
consumers are unaffected.
