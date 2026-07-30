# Longhorn

Shared Rust and Svelte/TypeScript systems for Tauri desktop applications.

Longhorn extracts proven desktop mechanisms from existing apps, separates
product policy from reusable behavior, and supplies composable pieces for new
projects. It complements Poodle: Poodle owns visual primitives; Longhorn owns
desktop application state, host integration, and orchestration.

## Start Here

- [Docs](docs/README.md)
- [Vision](docs/vision/001-shared-tauri-systems.md)
- [Initial Tauri audit](docs/research/translation-memos/001-tauri-application-extraction-audit.md)
- [Shared system suite](docs/specs/001-shared-desktop-system-suite.md)
- [g01 roadmap suite](docs/roadmaps/g01/README.md)
- [Agent rules](AGENTS.md)

## Default Effigy Loop

```sh
effigy tasks
effigy doctor
effigy test --plan
effigy qa
```

## Current State

Strict-paused Northstar docs spine installed. The five-app audit, promoted
foundation contracts, package topology, and full g01 runway are preserved.
The configuration domain store, coordinated atomic patch mutation, bounded
debounce, explicit flush, and coordinated backup capture are implemented.
Deterministic ZIP publication, bounded inspection, and safe retention are
implemented. Non-mutating restore inspection, exact conflict planning,
private staging, journaled publication, exact rollback, crash recovery,
coordinated load-sets, safe migration rewrite, and optional binary age v1
envelopes are implemented. Capability-declared custom adapters, truthful
external consistency groups, explicit adapter restore receipts, SQLite native
snapshot proof, and Loophole/Soundcheck/Bovine conformance fixtures are
implemented. Cross-platform storage identity, native/unified/portable layouts,
fixed bootstrap selection, journaled profile transition, receipt-bound source
cleanup, legacy discovery, root provenance, and Tauri-supplied path mapping
are implemented. `g01.002` is complete. `longhorn-core` now provides typed
physical, screen-DIP, and client-CSS geometry, checked scale conversion,
clamping, opaque display/window ids, and distinct desired/live window frames.
`longhorn-display` now provides persistent known-display records,
evidence-precedence correlation, explicit ambiguity, injected identity,
availability, labels, and deterministic arrangement signatures.
`longhorn-windowing` now resolves configured homes, ordered fallbacks,
intersection/main/deterministic recovery, fitted normal geometry, maximized
state, and settled-placement proposals. It also plans deterministic
desired/live create, retag, geometry, visibility, focus, and close operations
with protected-primary policy, host capabilities, and apply generations.
Cards 013-016 and `g01.003` are complete. Tauri host evidence from Loophole,
Nucleus, and Soundcheck is promoted. `longhorn-tauri-windowing` now provides
checked Tauri 2 display and explicitly managed-window observation,
process-local display metadata, complete raw physical snapshots, exact primary
matching, and whole-desktop logical mapping. It now also provides a strict
managed registry, consumer-owned dynamic creation, ordered main-thread native
mutation, partial receipts, apply evidence, and fresh convergence readback.
`longhorn-windowing` also provides a pure native-event lifecycle
coordinator with explicit attribution, user precedence, settling, persistence
debounce, bounded flush, close, and destroy directives. Cards 017-019 are
complete. `longhorn-tauri-windowing` now adds complete settled capture,
schema-opaque injected persistence, bounded flush receipts, consumer
page-readiness reveal gating, listener installation, and aggregate shutdown.
Card 020 is complete. Card 021 adds runtime-generic host composition, mock
proofs, narrow capabilities, fault evidence, and idempotent teardown. Card 022
adds the Rust 1.85-compatible graph and packaged macOS arm64 proof.
`g01.004` is complete. The layout donor refresh is promoted through research
memo 009 and contract 014. `longhorn-layout` now provides bounded identities,
registered schemas and panel policy, durable container/region/panel state,
fixed-point sizing, deterministic validation and normalization, and derived
empty-region visibility. Loophole eight-region and Nucleus five-region shapes
pass through the same Surface-independent model. Card 023 is complete. Card
024 adds atomic expected-revision create, close, activate, reorder, move,
sizing, and collapse mutation with exact failure invariance and opt-in bounded
request replay. Card 025 adds registered persistence, registry-digest
migration policy, fresh coordinated publication, and bounded presentation
debounce. Card 026 adds checked Rust-generated TypeScript protocol,
compatibility guards, golden fixtures, exact helpers, and package validation.
Card 027 adds checked Loophole and Nucleus conformance through one resolver and
mutation engine, exact TypeScript snapshot parity, and package-boundary
evidence. Cards 023-027 and `g01.005` are complete. Read-only Loophole
revalidation is promoted through research memo 010 and revised contracts 002
and 011. The first transfer line separates optional Surface state from the
no-Surface path, limits panel moves to one registered layout document, and
makes empty-display window provisioning explicit and receipted. Cards 028-035
compile the g01.006 runway. Card 028 adds bounded Surface identity, strict
topology, canonical normalization, consumer-resolved presence, and
deterministic available-window fallback in the optional pure
`longhorn-surfaces` crate. Card 029 adds expected-revision lifecycle, external
container inventory evidence, exact active fallback, explicit cleanup intent,
and registered coordinated persistence with migration, recovery, and backup
policy. Card 030 adds the optional pure Surface/window projection, composes it
through the existing runtime-generic host, proves missing and returning window
behavior, ordered shutdown, and Loophole and no-Surface Nucleus shapes. Card
031 adds exact injected transfer-session identity and monotonic time, finite
process-local registries, atomic complete drop-zone leases, client-epoch and
destroy invalidation, terminal single-use, and deterministic target
resolution. Card 032 adds fresh movable-panel admission, opaque direct-window
and Surface-container bindings, same-domain and revision rechecks, and the
existing coordinated expected-revision `MovePanel` publication with exact
abort invariance. Card 033 adds fresh whole-Surface admission, Surface-only
expected-revision moves, exact layout-binding retention, opt-in empty-display
policy, and receipted provision, cleanup, and host reconciliation. Card 034
adds checked Surface, transfer, and optional Surface-transfer protocols,
framework-neutral clients, listener-before-snapshot epoch handling, narrow
Tauri transport and handler assembly, managed-window geometry projection, and
audited capability examples. Card 035 adds passing direct and
Surface-enabled Rust 1.85 packaged macOS arm64 artifacts, real multi-webview
transfer, explicit empty-display provision, exact failure invariance, 2× scale
boundaries, and dependency, payload, capability, and authority audits.
`g01.006` is complete. Research memo 011 compiles g01.007 into Cards 036-041:
client lifetime, domain-free Tauri transport, Svelte state, a Poodle public
drag and preview-artifact checkpoint, public layout bindings, armed
cross-window drag, titlebar behavior, and three shell proofs. Card 036 is
complete with the shared checked client lifetime and domain-free Tauri
transport. Card 037 is complete with isolated Svelte state, consumer-fed layout
projection, optional domain subpaths, request-keyed optimism, and mounted
teardown. Card 038 is complete with Poodle's public typed drag seam and exact
preview artifact. Card 039 is complete with private Surface-free Poodle layout
bindings, consumer-owned presentation snippets, serialized revisioned
mutation, and mounted Nucleus and Loophole shapes. Card 040 is complete with
armed transfer, checked leases, compatible reveal, and injected titlebar
drag. Card 041 closes `g01.007` with isolated artifact-installed Bovine,
Nucleus, and Loophole shells. Research memo 012 and compiled contract 005 now
define the settings registry, one-domain apply units, policy/activation
projection, checked clients, public-Poodle shell, shared recovery pages, and
artifact proof. Cards 042-048 form g01.008. Card 042 now supplies the pure
sealed registry and authority protocol. Card 043 adds checked one-domain
configuration apply, policy enforcement, scoped reset, exact durability, and
post-publication activation. Card 044 adds generated TypeScript, checked
clients, registry helpers, and injected Tauri host assembly. Card 045 adds
isolated Svelte settings sessions and one public-Poodle shell for modal,
window, and panel hosts. Cards 046-047 add exact storage, backup, restore, and
recovery pages. Card 048 adds isolated artifact-installed Bovine, Soundcheck,
Loophole, and Nucleus compositions with exact dependency, capability,
authority, transaction, recovery, and UI audits. `g01.008` is complete.
Northstar is paused at the explicit post-008 intent checkpoint.
