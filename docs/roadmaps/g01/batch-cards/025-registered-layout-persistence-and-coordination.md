# 025 Registered Layout Persistence And Coordination

Status: complete
Owner: Tom
Roadmap: g01.005 batch 2
Governing refs: contracts 001, 004, 012, and 014; research memos 004, 005, and 009
Auto-start next card: no

## Objective

Persist authoritative layout documents through an injected registered
configuration domain without coupling layout state to window geometry or
consumer scope.

## Scope

- narrow `longhorn-layout-config` adapter crate
- consumer-injected descriptor, storage class, path, default, registry, and
  backup policy
- current schema codec and explicit migration hook
- definition-registry digest
- fresh-state expected-revision mutation under store coordination
- immediate structural publication
- bounded sizing/collapse debounce and explicit flush
- recovery, pending-intent, and publication receipts
- concurrent independent layout and window-domain proof

## Public Behavior

Longhorn does not choose one layout file or infer user, project, workspace, or
profile identity. The caller registers an exact configuration descriptor for
one layout aggregate.

Every mutation loads fresh coordinated state, rechecks revision, applies the
Card 024 engine, and publishes one complete layout document. Structural
commands are immediate. Consumers may opt sizing and collapse into a bounded
debounce lane.

A registry-digest change requires explicit migration. Corrupt, future, or
policy-mismatched state remains inspectable and is never replaced by defaults.

The configuration envelope owns the schema version. Its raw layout value owns
the registry digest and complete layout document. A registry change without a
schema bump enters recovery. A schema bump reaches the new registry only
through the injected migration hook.

## Out Of Scope

- storage-profile policy already owned by `longhorn-config`
- cross-domain atomic transactions
- window geometry codecs
- project-id or user-id types
- renderer persistence
- TypeScript, Svelte, Poodle, Tauri, or donor writes

## Steps

1. Add the narrow adapter crate and dependency boundaries.
2. Define the registered layout domain and current document codec.
3. Bind and validate the definition-registry digest.
4. Expose explicit consumer descriptor and default construction.
5. Apply commands over fresh state inside config coordination.
6. Preserve stale and rejected mutations without publication.
7. Publish successful structural commands immediately.
8. Add bounded sizing/collapse debounce over the same domain.
9. Add explicit per-domain and aggregate flush.
10. Preserve pending intent and evidence across failures.
11. Prove separate layout and window descriptors cannot overwrite each other.
12. Add backup-policy and migration-hook fixtures.

## Acceptance Criteria

- no default path, storage class, or scope is inferred
- invalid descriptor or unregistered domain fails typed
- fresh-state expected revision prevents lost layout updates
- successful mutation publishes one valid current document
- rejected mutation creates no file and changes no bytes
- registry mismatch never silently reinterprets state
- structural commands do not wait behind presentation debounce
- sizing/collapse debounce is bounded and explicitly flushable
- failed debounce retains the same pending generation
- layout publication cannot replace window geometry
- backup participation is explicit and descriptor-bound
- adapter graph contains config and layout but no host or UI package

## Evidence Required

- missing/current/migrated/recovery load fixtures
- registry-digest mismatch fixtures
- two-store concurrent mutation fixture
- structural-versus-debounced ordering fixture
- flush timeout and failure-retention fixtures
- separate layout/window domain concurrency fixture
- backup catalogue fixture
- Rust 1.85 and full Effigy QA

## Stop Conditions

- layout must choose a consumer storage scope or filename
- window geometry must share one decoded document
- policy mismatch must be silently normalized
- structural mutation must depend on renderer debounce
- a Tauri, Svelte, Poodle, Surface, or product type enters the adapter

## Outcome

- added the narrow `longhorn-layout-config` crate
- kept descriptor, storage class, path, default, registry, migration, and
  backup policy consumer-injected
- added a canonical SHA-256 registry digest over limits and definitions
- added a strict current raw value containing registry digest and document
- preserved current-schema registry mismatch as exact recovery evidence
- required schema bump plus explicit hook for registry migration
- applied Card 024 requests inside fresh coordinated config mutation
- returned combined layout and atomic publication receipts
- limited debounce intents to ordered sizing and collapse requests
- reused bounded pending weight, explicit retry, per-lane flush, and aggregate
  flush from `longhorn-config`
- proved layout and window descriptors remain independent under concurrency

## Evidence

- 18 adapter tests cover missing, current, corrupt, future, migrated, missing
  migration, and registry-mismatch loads
- same-revision two-store mutation admits one publication and one stale
  rejection
- stale rejection preserves exact published bytes
- structural publication commits while presentation intent remains pending
- ordered sizing/collapse intent publishes one complete successor document
- bounded-stage rejection preserves the existing generation
- helper-process lock timeout retains the same pending generation and retries
  successfully
- aggregate flush covers two independently registered layout domains
- backup catalogue fixture proves explicit include and exclusion policy
- independent layout/window mutations preserve both files
- Rust 1.85 config/layout/adapter tests pass
- current-toolchain warnings-denied Clippy passes
- Rust 1.85 workspace all-target check and full Effigy QA pass
- dependency graph contains core, config, layout, serde, and no host/UI crate

## Posture

`strict-ready`

## Next Task

Cards 026-027 and `g01.005` are complete. Card 028 is ready under the compiled
`g01.006` runway.
