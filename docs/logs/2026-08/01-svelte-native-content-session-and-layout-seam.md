# Svelte Native-content Session And Layout Seam

Date: 2026-08-01
Card: 092
Roadmap: g01.018

## Result

Added `@inflatable-cookie/longhorn-native-content-svelte` as a separate package over the checked
framework-neutral client. Each `NativeContentSession` creates one connection
per mount. It measures one exact consumer-supplied element and combines that
measurement with explicit scale, final visibility policy, focus intent, and
input routing.

Host window, attach generation, rounding, and presence always come from the
current checked snapshot. Renderer measurement remains desired-state input;
only host admission makes it durable.

## Lifetime And Update Policy

One serialized pump handles viewport, scale, visibility, focus, and routing
changes. Changes arriving during a pending update collapse into one later
update over fresh connection state. Current lifecycle and attach generation
gate completion. A late result from an older generation or mount cannot
replace the current projection.

Stop invalidates the lifecycle before disposal, disconnects the owned
`ResizeObserver`, disposes the checked listener-backed connection, and clears
the authoritative projection. Remount creates a fresh client epoch. The
package imports safely during SSR and never reads ambient device scale.

Consumers may resolve ordered explicit visibility inhibitors with
`resolveNativeContentVisibility`; the session accepts only the final policy.
It does not discover overlays or infer occlusion.

## Public Layout Seam

The proof fixture places a consumer-owned viewport element inside public
Poodle `Surface` children and binds the action to that element. Poodle is a
test-only consumer dependency. Production source and metadata contain no
Poodle edge, private selector, ancestor query, device-scale read, or semantic
input handler.

The same session fixture covers child-view `native_direct` and backing-surface
`renderer_forwarded` composition. Mechanism behavior remains outside the
Svelte adapter.

## Validation

- TypeScript and Svelte checks pass with zero diagnostics
- nine client, SSR, lifecycle, stale-result, teardown, policy, and package
  tests pass
- dry-run package output contains only metadata, README, and five source files
- focused Northstar path and dependency/DOM-authority checks pass

## Next Task

Execute Card 093. Prove produced graphs and three-shape conformance, reconcile
support claims, record exact migration prerequisites, and close g01.018.
