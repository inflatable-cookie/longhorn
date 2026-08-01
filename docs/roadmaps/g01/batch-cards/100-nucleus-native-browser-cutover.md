# 100 Nucleus Native Browser Cutover

Status: ready
Owner: Tom
Roadmap: g01.014 batch 4
Governing refs: contracts 003, 010, 012, 013, and 017; Cards 089, 092-095, and 099
Depends on: Card 099
Auto-start next card: no

## Objective

Replace Nucleus Browser viewport and child-webview coordination with the
production native-content kernel, Tauri child-view adapter, checked client,
and Svelte session while retaining Nucleus browser policy.

## Repository Scope

- Nucleus Browser host, renderer, capabilities, and focused tests may change.
- Longhorn may receive browser-policy conformance fixtures and migration
  evidence.

## Scope

- panel-to-island identity and attach generations
- explicit parent binding, source, viewport, scale, visibility, and focus
- Tauri child construction, reuse, observation, close, and host teardown
- checked renderer protocol and per-instance Svelte measurement
- Nucleus navigation, popup, download, permission, data-store, toolbar, notice,
  cursor, and system-open policy adapters
- exact remote-child capability isolation
- superseded child coordination and raw Tauri renderer API removal

## Steps

1. Freeze the Card 094 Browser trace and map panel ids to island ids.
2. Register the Nucleus construction and navigation policy adapter.
3. Assemble the child-view host and checked native-content protocol.
4. Mount one Svelte session on the exact Browser viewport element.
5. Feed explicit scale, final visibility, focus, and parent binding.
6. Retain toolbar, notices, system open, cursor, and browser security policy.
7. Exercise attach, reuse, replace, stale completion, close, and host teardown.
8. Audit remote capabilities, labels, data store, schemes, popup, and download.
9. Remove raw renderer Webview calls and duplicate coordination paths.
10. Run the packaged macOS Nucleus Browser smoke and rollback drill.

## Acceptance Criteria

- one current child exists per Browser panel and replacement advances generation
- listeners precede snapshot/attach and stale completions cannot reveal a child
- measured bounds use the exact consumer viewport and explicit scale
- final visibility matches active, mounted, nonzero, gesture, and overlay policy
- HTTP/HTTPS normalization and navigation admission remain exact
- popup and download requests stay denied with trusted-chrome notices
- normal shared engine data-store behavior remains explicit; Nucleus stores no
  cookies or credentials
- remote content receives no Nucleus or Longhorn Tauri capability
- panel close destroys; unmount/project switch hides for process-lifetime reuse
- host destruction and session stop are idempotent and observable
- old `Webview` renderer calls and duplicate coordination paths are removed

## Evidence Required

- native/renderer policy trace
- capability and remote-IPC negative audit
- attach, replacement, stale, visibility, focus, close, and teardown receipts
- popup, download, scheme, data-store, and cursor tests
- packaged macOS Nucleus Browser smoke
- dependency and duplicate-code inventory

## Stop Conditions

- a browser or product payload must enter the shared protocol
- remote content requires Nucleus capabilities
- current data-store behavior cannot be reproduced explicitly
- final visibility is unavailable without private Poodle inspection
- packaged behavior regresses from Card 089 support claims
- donor worktree changes overlap Browser files

## Next Task

Execute Card 101. Prove the complete migration, remove remaining duplicate
mechanisms, and transfer authority only where conformance passes.
