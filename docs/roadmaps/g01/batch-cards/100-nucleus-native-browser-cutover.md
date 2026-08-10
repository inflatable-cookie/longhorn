# 100 Nucleus Native Browser Cutover

Status: completed
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

- [x] native/renderer policy trace
- [x] capability and remote-IPC negative audit
- [x] attach, replacement, stale, visibility, focus, close, and teardown receipts
- [x] popup, download, scheme, data-store, and cursor tests
- [x] packaged macOS Nucleus Browser smoke
- [x] dependency and duplicate-code inventory

## Completion

Nucleus commit `ce71af24c7f042c16d0f44ee4f13332cb4fdfa98`
uses one native-content host and child-view adapter per stable Browser island,
one checked renderer client and Svelte session per mount, exact viewport and
scale input, explicit final visibility, retained generation across ordinary
unmount, destroy on panel close, and aggregate host teardown.

Longhorn commit `920bb8c1f19e0ce3a6a5766598d2bfa488ddde63`
adds bounded native-only construction and trusted-chrome hooks. Nucleus keeps
HTTP/HTTPS admission, popup/download notices, normal shared engine data-store
choice, toolbar, system open, cursor policy, and remote-capability closure.
No browser payload enters the shared renderer protocol.

The raw renderer Webview import and ensure/bounds/show/hide/close commands are
removed. The main capability drops raw Webview permissions and still matches
only `main`. An isolated macOS release bundle proves attach, physical bounds,
show, hide on tab switch, generation-1 reuse without reattach, close, overlay
precedence, URL policy, and Example Domain rendering. The exact fixture and
static proof live under `fixtures/migration/nucleus-card100`. The verifier was
removed 2026-08-10 — Longhorn no longer keeps consumer-aware proofs — so the
fixture is the retained evidence.

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

## Execution Note

Card 100 started from clean Nucleus and Longhorn `main` worktrees. The frozen
policy audit confirms that native-content may own island lifecycle, generation,
geometry, visibility, focus, and teardown while Nucleus retains URL handling,
trusted notices, toolbar commands, data-store choice, remote capability policy,
and the allowlisted macOS cursor bridge. Consumer policy callbacks remain on the
native side and do not enter the shared renderer protocol.
