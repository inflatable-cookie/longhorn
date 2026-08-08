# Compose Desktop Systems

Status: checked private adoption guidance
Updated: 2026-08-04
Governing contracts: [002-017](../contracts/contract-index.md)

## Why This Matters

Every Longhorn system draws the same line: Longhorn owns the mechanism, the
app owns the product policy. Compose in the order below and that line stays
clear — renderer state never becomes durable truth, and teardown never leaves
native owners behind. Get the order wrong and you get silent fallbacks, lost
config, or ghost windows. Terms are defined in the [glossary](glossary.md).

## Composition Order

Build downward from product authority:

1. register application identity, storage profile, and configuration domains
2. create pure domain authorities and product adapters
3. add Tauri handlers and capabilities only for selected operations
4. create framework-neutral clients
5. create one Svelte session per mounted host
6. compose public Poodle primitives with consumer content
7. install listeners before authoritative snapshots
8. reveal only after required authority is ready
9. flush, stop sessions, release leases, and tear down native owners explicitly

No renderer projection, visible control, Tauri permission, event, or cached
snapshot becomes durable or product authority.

## System Boundaries

| System | Longhorn owns | App retains | Add-on boundary |
| --- | --- | --- | --- |
| configuration | typed roots, registered files, safe mutation, backup/restore/recovery | schemas, defaults, retention, user recovery policy | age, custom database/secure-store adapters |
| settings | sealed registry, apply/session protocol, shared storage pages | product pages, renderers, multi-domain transactions | Svelte and Poodle shell |
| display/window | typed coordinates, inventory correlation, placement/apply/lifecycle plans | window definitions, creation policy, product readiness | Tauri observation/execution and config persistence |
| layout | registered containers, regions, panels, sizing, mutation, persistence | panel bodies, product metadata, layout definitions | Svelte/Poodle projection |
| Surface | optional host/container topology and lifecycle | product Surface definitions and presence policy | persistence and window projection |
| transfer | bounded sessions, leases, target resolution, checked commit | eligibility, product drag affordance, direct-window policy | optional whole-Surface feature |
| commands | sealed registry, search, keymap resolution, fresh admission | catalogue, availability, authorization, typed execution | config, Tauri metadata, Svelte/Poodle/settings |
| bridge | session, compatibility, authority descriptors, ordering, retry classes | domain operations, payloads, transport/security/service policy | Tauri events and injected supervision |
| history | linear structure, navigation plans, retention, metadata | payloads, atomic apply, codec, snapshot/journal, project versions | Tauri metadata and Svelte/Poodle panel |
| operations | finite lifecycle, progress, cancellation receipts, retry lineage | executor, scheduler, work payload, durable restart | Tauri or typed bridge transport |
| notifications | retained ledger, seen/dismiss/replace/prune, bounded projection | wording, publication policy, semantic actions | optional operation observation |
| native content | desired/observed coordination, generation, geometry, gates | browser/plugin/GPU content and semantic input | exactly one mechanism plus optional Svelte lifetime |

## Window, Layout, Surface, And Transfer

Layout is Surface-independent. A workspace may stop at:

```text
window → layout container → region → panel
```

Full hosting adds an optional layer:

```text
display → window → Surface → layout container → region → panel
```

Surfaces hold external container bindings; layout holds registered structure.
Presence is injected evidence. Missing hosts resolve through explicit policy.
Window provisioning, cleanup, fallback, and shutdown are planned and
receipted.

Panel transfer operates inside one registered layout document. Whole-Surface
transfer is a separate package and host feature. Native drag payloads contain
only protocol version and host-issued session id. They never serialize panels,
Surfaces, layouts, windows, bindings, or product data. Poodle owns drag
interaction; Longhorn uses its public extension points.

See [App Shell Composition](../architecture/app-shell-composition.md) and the
[greenfield workspace/full-hosting roots](../../examples/greenfield-compositions/README.md).

## Settings

Register stable modules, pages, renderer ids, scopes, apply units, and
capabilities, then seal. One mounted modal, window, or panel owns one session.
The consumer resolves renderer ids to product Svelte content.

Immediate and staged are interaction timing, not activation guarantees. One
apply unit may be atomic through its configured authority. Several units are
not one atomic save unless the consumer supplies a broader transaction.
Managed values remain non-writable; conflicts refresh authority.

Register shared Storage, Backups, and Restore pages only when their exact
operations and capabilities exist. See [Settings Composition](../architecture/settings-composition.md).

## Commands, Keymaps, And Palette

Use one sealed catalogue generation, availability snapshot, and effective
keymap across keyboard, palette, menus, help, and settings. Persist only active
preset identity and sparse directives. Execution always rechecks current
context, capability, arguments, and availability before calling an injected
product executor.

A command id is not a Tauri command or bridge operation name. Shared Tauri
handlers expose catalogue/keymap reads and mutation, not generic execution.
Palette visibility is never authorization. See
[Command System Composition](../architecture/command-system-composition.md).

## Optional Server Topology

Local domains remain independently authoritative. Select bridge only for a
real domain delivery seam. Session startup checks connection, host/session
identity, exact protocol, transport features, capability, and per-domain
authority separately.

Queries may use a bounded retry policy. Commands retry only with a durable
idempotency key and advertised deduplication; uncertain non-idempotent writes
are indeterminate. Listener-based projections subscribe before snapshot and
resync on gap or epoch change.

Supervision is injected lifecycle only. The app owns process/service choice,
transport, endpoint discovery, authentication, credentials, pairing,
installation, updates, and remote ownership. Longhorn currently claims no
production HTTP, WebSocket, socket, pipe, or remote provisioning layer. See
[Bridge Topology Composition](../architecture/bridge-topology-composition.md).

## Linear History

Apply the product mutation first, then record the applied typed payload. Undo,
redo, and checkout use plan → consumer atomic apply → checked structural
commit. Structural movement never precedes successful product apply.

Longhorn persists the bounded linear structure. The app persists payload
codec, canonical product snapshot, journal, fsync/autosave policy, recovery,
and project versions. Renderer metadata never contains product payload.

Fork-tree semantics are promoted planning evidence but remain outside current
production packages. Do not imply branch clients, checkpoints, or durable tree
availability. See [History Composition](../architecture/history-composition.md).

## Operations And Notifications

Select independently. Operation cancellation acceptance means the executor was
asked to stop; only a terminal transition means work ended. Renderer teardown
stops observation and never cancels host work.

Notifications are a separate retained ledger. Optional operation observation
is failure-isolated: publication failure cannot change operation truth. Toast
expiry affects transient presentation only. Semantic action references are
data and require fresh consumer authorization when invoked.

See [Operation And Notification Composition](../architecture/operation-notification-composition.md).

## Native Content

Select the pure kernel plus exactly the mechanism the product owns:

| Mechanism | Consumer-owned content | Current host evidence |
| --- | --- | --- |
| Tauri child view | browser construction, navigation/security/data-store policy | macOS first; packaged proof, live scale switch unavailable, focus/visibility may be unknown; Windows/Linux unproved |
| isolated window | helper/process, plugin ABI, authorization, native content | macOS packaged 11/11; Windows/Linux unsupported |
| backing surface | native storage, renderer/GPU, clipping execution, semantic input | macOS packaged 10/11; live scale transition unavailable; Windows/Linux unsupported |

The pure coordination code has deterministic 1x/2x semantics; Windows/Linux
target artifacts remain pending. Outer-window placement stays with windowing.
No raw native pointer, browser/plugin/GPU payload, or private Poodle DOM crosses
the shared renderer protocol. See
[Native-content Island Composition](../architecture/native-content-island-composition.md).

## Startup And Teardown

Required host state is explicit: loading, ready, reconnecting, unsupported, or
failed. Reveal follows checked authoritative load and the Svelte render
boundary. Missing capability never selects a fabricated local fallback.

Shutdown order follows ownership:

1. stop new UI admission
2. cancel armed transfers and release leases
3. stop per-instance client sessions and timers
4. await required config/history durability or report failure
5. detach or terminate native-content owners under declared policy
6. close dynamic then protected windows through the host plan
7. release storage/service authorities

Destructors perform no hidden I/O. Timeouts, publication failures, and partial
native teardown remain visible host decisions.

## Best-effort Diagnostics

Longhorn deliberately tolerates some failures — changed-event emit hints,
mutation-hint emits, native-content adapter teardown, terminal restore
journal cleanup — because the owning operation must not fail with them. The
`longhorn-core` diagnostics seam makes that class observable without
changing behavior.

Install one sink at composition time, before hosts are assembled:

```rust
use std::sync::Arc;

use longhorn_core::{BestEffortDiagnostics, install_best_effort_diagnostics};

struct AppDiagnostics;

impl BestEffortDiagnostics for AppDiagnostics {
    fn best_effort_failure(&self, area: &'static str, detail: &str) {
        // Route into the product's own logging or telemetry.
        eprintln!("longhorn best-effort failure at {area}: {detail}");
    }
}

fn install() {
    // The first installation wins; later calls return false and change
    // nothing, so library code can never displace the app's sink.
    let installed = install_best_effort_diagnostics(Arc::new(AppDiagnostics));
    debug_assert!(installed);
}
```

Rules:

- With no sink installed, behavior is exactly the historical silent
  tolerance; the seam is optional and adds no dependency.
- `area` values are stable dotted site names (for example
  `transfer.client-changed-emit`, `config.restore.journal-cleanup`,
  `native-content.child-view.close`); treat them as diagnostic labels, not
  a protocol.
- Reported failures were already tolerated: the owning operation has
  completed or failed on its own terms. Never turn a report into a retry
  loop against library internals; escalate through the operation's own
  typed receipts instead.
