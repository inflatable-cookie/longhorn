# Isolated-window Production Adapter

Date: 2026-08-01
Card: 090
Roadmap: g01.018

## Result

Added `longhorn-native-content-isolated-window` to the production workspace.
It executes current isolated-window plans through coordinator-validated
receipts over an injected owner runtime. The package owns generation checks,
bounded correlation, content-request admission, observation, partial apply,
and exact teardown evidence. Consumers retain process launch, native content,
authorization, transport I/O, and safe owner termination.

The strict helper protocol carries exact version `1` and has no outer-position
operation or field. It carries only content-area size, visibility, focus,
resize hints, observation, shutdown, and generation-bound acknowledgements.
Correlation uses bounded `NativeContentRequestId`; pending content requests
are capped at 128.

## Lifecycle And Size Policy

Callbacks install before owner launch. Duplicate correlation, stale/future or
retired generation, helper loss, timeout, and owner termination are distinct.
A timed-out teardown retains the handle for explicit retry. Cooperative close
or owner termination retires the generation and rejects later callbacks.

Host-driven size echoes are suppressed. Content-driven resize becomes a
generation- and desired-revision-bound proposal. Consumer policy accepts,
constrains, or rejects it; the decision receipt cannot rewrite durable desired
state.

## Packaged Evidence

The proof holds Card 084 infrastructure constant: the frozen same-binary
process runtime launches the packaged executable and creates a real controlled
AppKit `NSView`. A narrow bridge adapts that fixture to the production runtime
port. The production package itself has no prototype, Tauri, AppKit, plugin,
Signal, GPU, Svelte, or Poodle dependency.

The macOS 26.5.2 arm64 package is 9,484 KiB. Its executable SHA-256 is
`a3c2af69e55a79c226c0610ea00bd7f0b82eca1621247eb8a2106c70edbd23dd`.
All 11 retained checks pass:

- fake `NSView` attachment, bounded progress, and readiness
- host resize and cycle suppression
- constrained and rejected content resize decisions
- show, hide, focus loss, and resize-hint admission
- cooperative close
- consumer-owned outer recentering
- timeout followed by owner-process termination
- helper loss and stale-generation rejection
- platform and dependency boundary audit

Windows and Linux are explicitly unsupported. No cross-platform claim follows
from portable coordination source.

## Validation

- seven deterministic production adapter tests pass on Rust 1.85
- strict Clippy and formatting pass for the production crate and proof
- the packaged `.app` builds and its 74-event transcript has no proof failure
- the production dependency graph contains only core, native-content, serde,
  and derive dependencies
- inventory, target ledger, report, transcript, and focused Effigy checks pass

## Next Task

Execute Card 091. Implement backing-surface coordination over injected
storage, rendering, clipping, and input-gate ports without importing WGPU or
semantic input authority.
