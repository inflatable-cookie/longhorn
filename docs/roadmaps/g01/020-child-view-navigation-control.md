# g01.020 Child-view Navigation Control

Status: complete
Owner: Tom
Updated: 2026-08-03
Governing refs: contracts 001, 003, 012, and 017; g01.018; Figmatic g03.006
Depends on: g01.018 and Figmatic navigation-gap evidence

## Outcome

Let a retained Tauri child webview change documents through one
generation-checked, consumer-policy-admitted adapter operation without
recreating the view or adding browser data to the common native-content
protocol.

## Generation Runway

This is a bounded first-consumer correction inside g01. It reopens only the
child-view mechanism layer. Pure coordination, TypeScript/Svelte protocol,
isolated-window, backing-surface, and package publication remain unchanged.

## Execution Plan

### Batch 1. Authority and adapter semantics

- [x] [Card 132](batch-cards/132-child-view-navigation-contract-and-adapter-authority.md)
  freezes the child-specific boundary and implements exact policy, generation,
  idempotence, readiness, receipt, and failure semantics

### Batch 2. Native execution and packaged evidence

- [x] [Card 133](batch-cards/133-tauri-child-view-navigation-execution-and-packaged-proof.md)
  executes through the private Tauri handle and proves retained identity on a
  packaged macOS child

### Batch 3. Artifacts and consumer resume gates

- [x] [Card 134](batch-cards/134-child-view-navigation-artifact-closeout-and-consumer-handoffs.md)
  proves isolated artifacts, preserves the common protocol, and records exact
  Figmatic and Nucleus follow-ups

## Goals

- [x] keep URL and navigation policy consumer-owned
- [x] keep navigation out of `NativeContentOperation` and renderer protocols
- [x] retain one child handle and attach generation across document changes
- [x] reject stale and denied work before native mutation
- [x] make repeat navigation to the current URL a native no-op
- [x] expose submitted versus loaded as separate evidence
- [x] preserve optional graph and remote-capability closure

## Acceptance Criteria

- [x] allowed navigation changes the document without attach or close
- [x] same-URL navigation returns unchanged without reload
- [x] denied, stale, future, absent, observation, and native failures are exact
- [x] page-load start and finish drive not-ready and ready for the same generation
- [x] produced child-view artifacts contain the API without sibling source
- [x] pure Rust and TypeScript native-content artifacts remain unchanged
- [x] packaged macOS evidence proves retained navigation
- [x] Figmatic can resume planning without adopting a shell-page workaround

## Explicit Non-goals

- renderer-authorized URL commands
- browser history, back, forward, reload, redirects, or persistence
- URL state in the common desired/observed model
- Figmatic or Nucleus repository writes
- Windows/Linux support changes
- package-manager publication

## Next Task

No Longhorn card auto-starts. Resume Figmatic g03.006 using the consumer
handoff. Nucleus may schedule its separate adapter-control cleanup.
