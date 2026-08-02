# Loophole Renderer, Poodle, And Transfer Cutover

Date: 2026-08-02
Card: 109
Status: complete

## Changed

Aura now assembles Longhorn panel and whole-Surface adapters over one shared
Tauri transfer handler. Each renderer mounts one listener-first client epoch,
publishes complete measured panel-region and Surface-window leases, and tears
down sessions and leases on unmount.

All eight panel regions now use Poodle's public external drag source and drop
target contracts. Same-region reorder remains Poodle authority. Cross-region
and cross-window moves commit through the registered layout authority. Hidden
eligible regions exist only as renderer projection during an armed drag.

The old Loophole MIME, Poodle tab-id parsing, private class lookup, capture
handlers, and window-edge drag-out heuristic are removed. Native payloads use
the Longhorn protocol version and host-created session id only.

Whole-Surface moves between managed windows commit through the registered
Surface authority and trigger native host reconciliation. Empty-display
window creation remains explicit Loophole product policy: the old spawn path
runs only after the checked generic attempt returns
`empty_display_disabled`. This preserves current functionality without
claiming generic provisioning policy in Longhorn or Poodle.

Compatibility publication now preserves Aura's `surfacePriority` attachment
and disables an emptied source window. Focused-panel Surfaces, panel catalogue,
presentation, display fallback, and native window policy remain Loophole.

## Evidence

- Aura Svelte check: 0 errors, 0 warnings.
- Aura renderer suite: 101 files, 1,021 tests.
- Focused renderer transfer/policy tests: 13 passed.
- Aura production renderer build passed; only existing Vite chunk-size and
  mixed static/dynamic import warnings remain.
- Aura native compatibility projection tests: 2 passed.
- Aura Rust library check passed; only pre-existing dead-code warnings remain.
- Longhorn transfer TypeScript: 11 passed.
- Longhorn Surface transfer TypeScript: 6 passed.
- Longhorn Svelte transfer state/actions: 20 passed.
- Longhorn Tauri transfer handler/capabilities: 7 passed.
- Shared handler evidence covers epoch replacement, teardown, target loss,
  ambiguity, expiry, stale geometry, and scaled projection.
- Aura boundary audit rejects private Poodle MIME, DOM, class, and id
  discovery and proves both adapters share one handler epoch.

## Retained Boundary

Dynamic empty-display window creation is still Loophole's product adapter.
Moving it to generic provisioning would require a product-approved hidden
window preparation and readiness implementation. Card 109 does not invent
that policy or weaken current behavior.

## Next

Execute Card 110: settings, command discovery, keyboard resolution, palette,
and conflict projection.
