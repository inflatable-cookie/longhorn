# 124 Jetstream Migration Conformance And Closeout

Status: complete
Owner: Tom
Roadmap: g01.016 batch 4
Governing refs: contracts 003, 006-007, 009-010, 012-013, and 017;
Cards 113-114 and 122-123
Depends on: Card 123
Auto-start next card: no

## Objective

Prove Jetstream's bridge, command, and backing-surface adoption, remove
superseded generic copies, and preserve engine/editor authority.

## Repository Scope

- Longhorn and Jetstream: tests, evidence, duplicate cleanup, and canonical docs.
- Poodle and other consumers: read-only receipts.
- Package registries: unchanged.

## Scope

- fresh start, listener race, snapshot gap, reconnect, remount, and teardown
- command discovery, availability, keyboard, execution, and stale admission
- native attach, resize, viewport move/collapse, scale, input gates, and destroy
- no-surface diagnostic and unsupported-platform outcomes
- exact artifact, graph, peer, capability, duplicate, and retained-adapter inventories

## Steps

1. Reverify exact source and artifact receipts.
2. Run selected Longhorn and Jetstream editor suites.
3. Exercise bridge and command native/renderer parity.
4. Exercise backing-surface and viewport lifecycle matrices.
5. Audit capabilities, peers, optional edges, raw IPC, and generated bindings.
6. Remove superseded generic bridge, keyboard, and viewport-lifetime copies.
7. Verify retained engine/editor adapters and previous-build rollback.
8. Update authority maps, roadmaps, logs, and consumer evidence.

## Acceptance Criteria

- one bridge, command/keymap, and native-content coordination authority exists
- Jetstream installs no config, settings, layout, Surface, transfer, history,
  operation, notification, or isolated-window package
- full editor and isolated artifact checks pass
- product command, renderer, world, WGPU, and semantic-input authority remain local
- native support claims match packaged evidence
- no raw pointer, WGPU type, or editor payload enters Longhorn
- previous behavior remains recoverable without dual writes
- package-manager publication remains deferred

## Stop Conditions

- any behavior delta lacks contract and source evidence
- an active generic donor duplicate remains
- rollback requires a second authority
- closeout widens platform or compatibility claims

## Next Task

Execute Card 125. Build the four produced-artifact greenfield compositions.
