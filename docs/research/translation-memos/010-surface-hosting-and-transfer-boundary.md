# Surface Hosting And Transfer Boundary

Status: promoted
Owner: Tom
Updated: 2026-07-29

## Question

What Surface-hosting and drag behavior is actually proven by Loophole, and
what can g01.006 promise after the delivered Longhorn window host and layout
core?

## Repositories

- `loophole/echo`
- `loophole/aura`
- `nucleus/crates/nucleus-workspaces`
- `nucleus/apps/desktop`
- `longhorn`

Inspection was read-only. Donor worktrees were not modified.

## Loophole Surface Evidence

Current Loophole code proves:

- stable Surface ids separate from window ids
- optional labels
- ordered Surface membership per window
- one active Surface per window
- unique resolved ownership by one window
- create, duplicate, close, activate, reorder, and cross-window move
- preferred/fallback window and display behavior
- presence filtering before hosting resolution
- whole-Surface drag-out using a screen point
- host-side managed-window hit-testing
- optional new-window creation on empty display space

The donor also exposes boundaries not worth copying:

- Surface presence uses legacy product-shaped window clauses
- Surface, panel, window, and display configuration share one large document
- repair paths silently deduplicate ownership and prune placements
- duplicate clones Loophole-specific region and panel configuration
- screen-point hit-testing takes the first overlapping live window
- errors are mostly strings
- lifecycle requests do not carry expected revisions

Longhorn keeps the mechanism and rejects those authority shortcuts.

## Panel Drag Evidence

Loophole and Nucleus both prove same-webview panel movement:

- Poodle or HTML5 drag payloads
- allowed-region projection
- hidden compatible-region reveal
- cross-region move and tab reorder
- optimistic renderer feedback followed by host mutation

Current Loophole whole-Surface drag crosses windows. Current active Aura panel
drag remains local to one webview. Archived BroadcastChannel helpers are not
authoritative cross-window commit evidence.

Cross-window panel transfer is therefore new Longhorn behavior, not a donor
extraction claim.

## Delivered Longhorn Constraints

The completed foundation provides:

- typed `ScreenDip`, client geometry, live outer window bounds, and checked
  scale conversion
- stable managed `WindowId` identity and fresh complete readback
- hidden placement, readiness reveal, dynamic window creation, close, and
  teardown receipts
- Surface-independent layout containers
- expected-revision panel move with exact failure invariance
- registered layout persistence under one store coordinator
- generated TypeScript layout contracts

It does not provide:

- a Surface document or persistence adapter
- a multi-domain atomic mutation API
- layout-container create, clone, or delete commands
- client-rectangle to screen-space host projection
- a transfer session or drop-zone lease registry

The first transfer line must fit those facts.

## Promoted Decisions

### Surface state

- `longhorn-surfaces` is optional and owns Surface identity, external
  layout-container binding, hosting preferences, ordered membership, active
  Surface, resolution, and expected-revision lifecycle.
- Product presence predicates stay consumer-owned. Consumers inject the
  currently admitted Surface set.
- Duplicate copies generic Surface metadata only. Caller-supplied fresh ids
  and an existing target layout container are required.
- Close returns cleanup intent. It does not infer deletion of layout or product
  state.
- `longhorn-surfaces-config` persists the Surface document in a distinct
  registered domain.

### Transfer state

- `longhorn-transfer` owns bounded host-created sessions and complete
  replacement leases.
- Payloads contain only protocol version and an unguessable session id.
- Leases bind one renderer/client epoch to one managed window and expire under
  an injected clock.
- Overlapping eligible targets are ambiguous. Enumeration order never decides.
- A first terminal commit attempt consumes the session.

### Panel commit

- v1 supports move, not copy.
- Source and target must share one registered layout document.
- The existing `MovePanel` command is the atomic commit.
- Cross-document transfer fails before mutation until a multi-domain
  transaction contract exists.

### Surface commit

- whole-Surface transfer commits one Surface document and retains the bound
  layout container
- empty-display window creation is opt-in through an injected consumer
  provisioner
- failed commit after provisioning requires explicit cleanup evidence

## Contract Delta

Contract 011 previously implied:

- panel and Surface commits had the same transaction shape
- move and copy were both available
- arbitrary source and target documents could commit atomically
- a separate nonce was needed beside the session id

The promoted boundary narrows this:

- shared session and target mechanics; explicit subject adapters
- move only
- one registered layout document for panel transfer
- one unguessable single-use session id
- cross-document atomicity deferred, not faked

## Promotion

Promoted into:

- `../../architecture/system-architecture.md`
- `../../architecture/package-topology.md`
- `../../architecture/system-inventory.md`
- `../../contracts/002-composable-workspace-hosting.md`
- `../../contracts/011-cross-window-transfer.md`
- `../../roadmaps/g01/006-optional-surfaces-and-cross-window-drag.md`
- Cards 028 through 035
