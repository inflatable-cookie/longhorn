# 081 Operation And Notification Artifact Proof And Closeout

Status: complete
Owner: Tom
Roadmap: g01.012 batch 4
Governing refs: contracts 001, 003, 007, 010, 012, 013, 015, and 016;
research memo 016
Depends on: Card 080
Auto-start next card: no

## Objective

Prove minimal and rich operation/notification compositions from produced
artifacts, publish the canonical adoption guide, and close g01.012 without
claiming consumer migration.

## Scope

- packed TypeScript artifacts and offline Rust source inventories
- minimal operation-only direct consumer
- Soundcheck-shaped scan consumer
- Loophole-shaped queued render plus notification consumer
- independent non-operation notification consumer
- direct, Tauri, and bridge-domain semantic traces
- Svelte remount and multi-window evidence
- public-Poodle progress, status, panel, and toast evidence
- package, payload, capability, authority, and teardown audits
- operation/notification composition and migration guide
- g01.012 closeout and next-runway pointer

## Out Of Scope

- writes to Soundcheck, Loophole, Nucleus, Jetstream, or Bovine
- production registry publication
- production network transport
- durable operation scheduler or resume
- native OS notifications
- optional history-tree work

## Steps

1. Pack every new TypeScript package and optional subpath.
2. Build separate minimal and rich Rust graphs from private source inventories.
3. Install isolated minimal, Soundcheck-shaped, Loophole-shaped, and
   notification-only TypeScript consumers.
4. Compare native and renderer transition traces.
5. Exercise cancellation races, late events, retention, remount, and teardown.
6. Exercise seen, dismiss, toast expiry, operation projection, and actions.
7. Audit no-product-payload, no-global-authority, and optional dependencies.
8. Document direct, Tauri, bridge, Svelte, Poodle, and consumer-policy seams.
9. Record migration prerequisites without changing donor repositories.
10. Run full Effigy QA and close g01.012.

## Acceptance Criteria

- all consumers install only produced artifacts
- minimal operation graph omits notifications, bridge, Tauri, Svelte, Poodle,
  config, settings, command, and history
- notification-only graph omits operation and command packages
- Soundcheck and Loophole fixtures preserve their claimed lifecycle behavior
- direct, Tauri, bridge, and renderer traces match
- cancellation and late-event failure invariance passes
- retained notifications survive toast expiry and remount
- semantic actions require current consumer authorization
- no donor product payload crosses generic protocols
- composition guide names executor, queue, retry, retention, wording, action,
  and migration ownership
- all front doors agree and full Effigy QA passes

## Evidence Required

- packed artifact manifests and isolated install logs
- offline Rust dependency inventories
- native/renderer semantic trace comparison
- cancellation, retention, remount, teardown, and action evidence
- package, payload, capability, and authority audit
- canonical composition guide
- migration prerequisite table
- closeout log and full QA

## Stop Conditions

- artifact consumers resolve sibling source or undeclared peers
- operation-only consumers require notifications or bridge
- notification-only consumers require operation or command
- donor behavior requires product payload in a generic protocol
- native and renderer semantics diverge
- full QA fails from lane changes

## Next Task

Return to the g01 generation runway. Start g01.013 native-content-island
characterization unless the current checkpoint promotes a higher-priority
consumer migration.
