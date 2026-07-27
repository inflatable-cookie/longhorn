# g01.010 Command Registry, Keymaps, And Palette

Status: blocked on `g01.008` and `g01.009`  
Owner: Tom  
Updated: 2026-07-27  
Governing refs: contract 006

## Outcome

Extract Loophole's command/input strengths into a product-neutral registry and
shared projections.

## Batches

### 1. Command core

- registry, typed arguments, categories, context, capabilities
- availability and authority-side execution validation
- explicit outcome taxonomy

### 2. Input resolution

- normalized triggers and active contexts
- default keymaps, sparse overrides, precedence, conflicts
- text-input and platform-reserved chord gates
- configuration-domain persistence

### 3. Projections

- command palette search and execution
- menu and shortcut descriptors
- settings page and chord capture
- optional macro resolution

### 4. Conformance

- Loophole command subset
- Jetstream editing/viewport shortcut subset
- keyboard race and focus tests

## Acceptance

- every projection consumes the same command ids
- product commands register without Longhorn knowing their domain
- unavailable commands cannot execute through stale renderer state
- conflict and override behavior is deterministic

